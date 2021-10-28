use super::model::BcMediaIframe;
use super::model::*;
use err_derive::Error;
use nom::IResult;
use nom::{combinator::*, named, number::streaming::*, take, take_str};
use std::io::Read;

// PAD_SIZE: Media packets use 8 byte padding
const PAD_SIZE: u32 = 8;

/// The error types used during deserialisation
#[derive(Debug, Error)]
pub enum Error {
    /// A Nom parsing error usually a malformed packet
    #[error(display = "Parsing error")]
    NomError(String),
    /// An IO error such as the stream being dropped
    #[error(display = "I/O error")]
    IoError(#[error(source)] std::io::Error),
}
type NomErrorType<'a> = nom::error::Error<&'a [u8]>;

impl<'a> From<nom::Err<NomErrorType<'a>>> for Error {
    fn from(k: nom::Err<NomErrorType<'a>>) -> Self {
        let reason = match k {
            nom::Err::Error(e) => format!("Nom Error: {:?}", e),
            nom::Err::Failure(e) => format!("Nom Error: {:?}", e),
            _ => "Unknown Nom error".to_string(),
        };
        Error::NomError(reason)
    }
}

fn read_from_reader<P, O, E, R>(mut parser: P, mut rdr: R) -> Result<O, E>
where
    R: Read,
    E: for<'a> From<nom::Err<NomErrorType<'a>>> + From<std::io::Error>,
    P: FnMut(&[u8]) -> nom::IResult<&[u8], O>,
{
    let mut input: Vec<u8> = Vec::new();
    loop {
        let to_read = match parser(&input) {
            Ok((_, parsed)) => return Ok(parsed),
            Err(nom::Err::Incomplete(needed)) => {
                match needed {
                    nom::Needed::Unknown => std::num::NonZeroUsize::new(1).unwrap(), // read one byte
                    nom::Needed::Size(len) => len,
                }
            }
            Err(e) => return Err(e.into()),
        };

        if 0 == (&mut rdr)
            .take(to_read.get() as u64)
            .read_to_end(&mut input)?
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Read returned 0 bytes",
            )
            .into());
        }
    }
}

impl BcMedia {
    pub(crate) fn deserialize<R: Read>(r: R) -> Result<BcMedia, Error> {
        // Throw away the nom-specific return types
        read_from_reader(|reader| bcmedia(reader), r)
    }
}

fn bcmedia(buf: &[u8]) -> IResult<&[u8], BcMedia> {
    let (buf, magic) = verify(le_u32, |x| {
        matches!(
            *x,
            MAGIC_HEADER_BCMEDIA_INFO_V1
                | MAGIC_HEADER_BCMEDIA_INFO_V2
                | MAGIC_HEADER_BCMEDIA_IFRAME..=MAGIC_HEADER_BCMEDIA_IFRAME_LAST
                | MAGIC_HEADER_BCMEDIA_PFRAME..=MAGIC_HEADER_BCMEDIA_PFRAME_LAST
                | MAGIC_HEADER_BCMEDIA_AAC
                | MAGIC_HEADER_BCMEDIA_ADPCM
        )
    })(buf)?;

    match magic {
        MAGIC_HEADER_BCMEDIA_INFO_V1 => {
            let (buf, payload) = bcmedia_info_v1(buf)?;
            Ok((buf, BcMedia::InfoV1(payload)))
        }
        MAGIC_HEADER_BCMEDIA_INFO_V2 => {
            let (buf, payload) = bcmedia_info_v2(buf)?;
            Ok((buf, BcMedia::InfoV2(payload)))
        }
        MAGIC_HEADER_BCMEDIA_IFRAME..=MAGIC_HEADER_BCMEDIA_IFRAME_LAST => {
            let (buf, payload) = bcmedia_iframe(buf)?;
            Ok((buf, BcMedia::Iframe(payload)))
        }
        MAGIC_HEADER_BCMEDIA_PFRAME..=MAGIC_HEADER_BCMEDIA_PFRAME_LAST => {
            let (buf, payload) = bcmedia_pframe(buf)?;
            Ok((buf, BcMedia::Pframe(payload)))
        }
        MAGIC_HEADER_BCMEDIA_AAC => {
            let (buf, payload) = bcmedia_aac(buf)?;
            Ok((buf, BcMedia::Aac(payload)))
        }
        MAGIC_HEADER_BCMEDIA_ADPCM => {
            let (buf, payload) = bcmedia_adpcm(buf)?;
            Ok((buf, BcMedia::Adpcm(payload)))
        }
        _ => unreachable!(),
    }
}

fn bcmedia_info_v1(buf: &[u8]) -> IResult<&[u8], BcMediaInfoV1> {
    let (buf, _header_size) = verify(le_u32, |x| *x == 32)(buf)?;
    let (buf, video_width) = le_u32(buf)?;
    let (buf, video_height) = le_u32(buf)?;
    let (buf, _unknown) = le_u8(buf)?;
    let (buf, fps) = le_u8(buf)?;
    let (buf, start_year) = le_u8(buf)?;
    let (buf, start_month) = le_u8(buf)?;
    let (buf, start_day) = le_u8(buf)?;
    let (buf, start_hour) = le_u8(buf)?;
    let (buf, start_min) = le_u8(buf)?;
    let (buf, start_seconds) = le_u8(buf)?;
    let (buf, end_year) = le_u8(buf)?;
    let (buf, end_month) = le_u8(buf)?;
    let (buf, end_day) = le_u8(buf)?;
    let (buf, end_hour) = le_u8(buf)?;
    let (buf, end_min) = le_u8(buf)?;
    let (buf, end_seconds) = le_u8(buf)?;
    let (buf, _unknown_b) = le_u16(buf)?;

    Ok((
        buf,
        BcMediaInfoV1 {
            // header_size,
            video_width,
            video_height,
            fps,
            start_year,
            start_month,
            start_day,
            start_hour,
            start_min,
            start_seconds,
            end_year,
            end_month,
            end_day,
            end_hour,
            end_min,
            end_seconds,
        },
    ))
}

fn bcmedia_info_v2(buf: &[u8]) -> IResult<&[u8], BcMediaInfoV2> {
    let (buf, _header_size) = verify(le_u32, |x| *x == 32)(buf)?;
    let (buf, video_width) = le_u32(buf)?;
    let (buf, video_height) = le_u32(buf)?;
    let (buf, _unknown) = le_u8(buf)?;
    let (buf, fps) = le_u8(buf)?;
    let (buf, start_year) = le_u8(buf)?;
    let (buf, start_month) = le_u8(buf)?;
    let (buf, start_day) = le_u8(buf)?;
    let (buf, start_hour) = le_u8(buf)?;
    let (buf, start_min) = le_u8(buf)?;
    let (buf, start_seconds) = le_u8(buf)?;
    let (buf, end_year) = le_u8(buf)?;
    let (buf, end_month) = le_u8(buf)?;
    let (buf, end_day) = le_u8(buf)?;
    let (buf, end_hour) = le_u8(buf)?;
    let (buf, end_min) = le_u8(buf)?;
    let (buf, end_seconds) = le_u8(buf)?;
    let (buf, _unknown_b) = le_u16(buf)?;

    Ok((
        buf,
        BcMediaInfoV2 {
            // header_size,
            video_width,
            video_height,
            fps,
            start_year,
            start_month,
            start_day,
            start_hour,
            start_min,
            start_seconds,
            end_year,
            end_month,
            end_day,
            end_hour,
            end_min,
            end_seconds,
        },
    ))
}

fn bcmedia_iframe(buf: &[u8]) -> IResult<&[u8], BcMediaIframe> {
    named!(take4str( &[u8] ) -> &str, take_str!( 4 ) );
    let (buf, video_type_str) = verify(take4str, |x| matches!(x, "H264" | "H265"))(buf)?;
    let (buf, payload_size) = le_u32(buf)?;
    let (buf, _unknown_a) = le_u32(buf)?;
    let (buf, microseconds) = le_u32(buf)?;
    let (buf, _unknown_b) = le_u32(buf)?;
    let (buf, time) = le_u32(buf)?;
    let (buf, _unknown_c) = le_u32(buf)?;
    let (buf, data_slice) = take!(buf, payload_size)?;
    let pad_size = match payload_size % PAD_SIZE {
        0 => 0,
        n => PAD_SIZE - n,
    };
    let (buf, _padding) = take!(buf, pad_size)?;

    let video_type = match video_type_str {
        "H264" => VideoType::H264,
        "H265" => VideoType::H265,
        _ => unreachable!(),
    };

    Ok((
        buf,
        BcMediaIframe {
            video_type,
            // payload_size,
            microseconds,
            time,
            data: data_slice.to_vec(),
        },
    ))
}

fn bcmedia_pframe(buf: &[u8]) -> IResult<&[u8], BcMediaPframe> {
    named!(take4str( &[u8] ) -> &str, take_str!( 4 ) );
    let (buf, video_type_str) = verify(take4str, |x| matches!(x, "H264" | "H265"))(buf)?;
    let (buf, payload_size) = le_u32(buf)?;
    let (buf, _unknown_a) = le_u32(buf)?;
    let (buf, microseconds) = le_u32(buf)?;
    let (buf, _unknown_b) = le_u32(buf)?;
    let (buf, data_slice) = take!(buf, payload_size)?;
    let pad_size = match payload_size % PAD_SIZE {
        0 => 0,
        n => PAD_SIZE - n,
    };
    let (buf, _padding) = take!(buf, pad_size)?;

    let video_type = match video_type_str {
        "H264" => VideoType::H264,
        "H265" => VideoType::H265,
        _ => unreachable!(),
    };

    Ok((
        buf,
        BcMediaPframe {
            video_type,
            // payload_size,
            microseconds,
            data: data_slice.to_vec(),
        },
    ))
}

fn bcmedia_aac(buf: &[u8]) -> IResult<&[u8], BcMediaAac> {
    let (buf, payload_size) = le_u16(buf)?;
    let (buf, _payload_size_b) = le_u16(buf)?;
    let (buf, data_slice) = take!(buf, payload_size)?;
    let pad_size = match payload_size as u32 % PAD_SIZE {
        0 => 0,
        n => PAD_SIZE - n,
    };
    let (buf, _padding) = take!(buf, pad_size)?;

    Ok((
        buf,
        BcMediaAac {
            // payload_size,
            data: data_slice.to_vec(),
        },
    ))
}

fn bcmedia_adpcm(buf: &[u8]) -> IResult<&[u8], BcMediaAdpcm> {
    const SUB_HEADER_SIZE: u16 = 4;

    let (buf, payload_size) = le_u16(buf)?;
    let (buf, _payload_size_b) = le_u16(buf)?;
    let (buf, _magic) = verify(le_u16, |x| *x == MAGIC_HEADER_BCMEDIA_ADPCM_DATA)(buf)?;
    // On some camera this value is just 2
    // On other cameras is half the block size without the header
    let (buf, _half_block_size) = le_u16(buf)?;
    let block_size = payload_size - SUB_HEADER_SIZE;
    let (buf, data_slice) = take!(buf, block_size)?;
    let pad_size = match payload_size as u32 % PAD_SIZE {
        0 => 0,
        n => PAD_SIZE - n,
    };
    let (buf, _padding) = take!(buf, pad_size)?;

    Ok((
        buf,
        BcMediaAdpcm {
            // payload_size,
            // block_size,
            data: data_slice.to_vec(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::Error;
    use crate::bc_protocol::filesub::FileSubscriber;
    use crate::bcmedia::model::*;
    use env_logger::Env;
    use log::*;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    fn init() {
        let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info"))
            .is_test(true)
            .try_init();
    }

    fn sample(name: &str) -> PathBuf {
        let dir = std::env::current_dir().unwrap(); // This is crate root during cargo test
        dir.join("src").join("bcmedia").join("samples").join(name)
    }

    #[test]
    // This method will test the decoding on swann cameras output
    //
    // Crucially this contains adpcm
    fn test_swan_deser() {
        init();

        let mut subsciber = FileSubscriber::from_files(vec![
            sample("video_stream_swan_00.raw"),
            sample("video_stream_swan_01.raw"),
            sample("video_stream_swan_02.raw"),
            sample("video_stream_swan_03.raw"),
            sample("video_stream_swan_04.raw"),
            sample("video_stream_swan_05.raw"),
            sample("video_stream_swan_06.raw"),
            sample("video_stream_swan_07.raw"),
            sample("video_stream_swan_08.raw"),
            sample("video_stream_swan_09.raw"),
        ]);

        // Should derealise all of this
        loop {
            let e = BcMedia::deserialize(&mut subsciber);
            match e {
                Err(Error::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                    // Reach end of files
                    break;
                }
                Err(e) => {
                    error!("{:?}", e);
                    panic!();
                }
                Ok(_) => {}
            }
        }
    }

    #[test]
    // Tests the decoding of an info v1
    fn test_info_v1() {
        init();

        let mut subsciber = FileSubscriber::from_files(vec![sample("info_v1.raw")]);

        let e = BcMedia::deserialize(&mut subsciber);
        assert!(matches!(
            e,
            Ok(BcMedia::InfoV1(BcMediaInfoV1 {
                video_width: 2560,
                video_height: 1440,
                fps: 30,
                start_year: 121,
                start_month: 8,
                start_day: 4,
                start_hour: 23,
                start_min: 23,
                start_seconds: 52,
                end_year: 121,
                end_month: 8,
                end_day: 4,
                end_hour: 23,
                end_min: 23,
                end_seconds: 52,
            }))
        ));
    }

    #[test]
    fn test_iframe() {
        init();

        let mut subsciber = FileSubscriber::from_files(vec![
            sample("iframe_0.raw"),
            sample("iframe_1.raw"),
            sample("iframe_2.raw"),
            sample("iframe_3.raw"),
            sample("iframe_4.raw"),
        ]);

        let e = BcMedia::deserialize(&mut subsciber);
        if let Ok(BcMedia::Iframe(BcMediaIframe {
            video_type: VideoType::H264,
            microseconds: 3557705112,
            time: 1628085232,
            data: d,
        })) = e
        {
            assert_eq!(d.len(), 192881);
        } else {
            panic!();
        }
    }

    #[test]
    fn test_pframe() {
        init();

        let mut subsciber =
            FileSubscriber::from_files(vec![sample("pframe_0.raw"), sample("pframe_1.raw")]);

        let e = BcMedia::deserialize(&mut subsciber);
        if let Ok(BcMedia::Pframe(BcMediaPframe {
            video_type: VideoType::H264,
            microseconds: 3557767112,
            data: d,
        })) = e
        {
            assert_eq!(d.len(), 45108);
        } else {
            panic!();
        }
    }

    #[test]
    fn test_adpcm() {
        init();

        let mut subsciber = FileSubscriber::from_files(vec![sample("adpcm_0.raw")]);

        let e = BcMedia::deserialize(&mut subsciber);
        if let Ok(BcMedia::Adpcm(BcMediaAdpcm { data: d })) = e {
            assert_eq!(d.len(), 244);
        } else {
            panic!();
        }
    }
}
