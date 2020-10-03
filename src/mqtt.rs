// This is because librumqttd requires jemalloc but jemalloc
// does not support msvc.
// I have an issue report here: https://github.com/bytebeamio/rumqtt/issues/161
//
#[cfg(not(target_env = "msvc"))]
pub use self::live::*;
#[cfg(not(target_env = "msvc"))]
mod live;

#[cfg(target_env = "msvc")]
pub use self::dummy::*;
#[cfg(target_env = "msvc")]
mod dummy;
