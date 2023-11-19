use d3xs_protocol::crypto;

#[cfg(not(target_os = "espidf"))]
pub type Random = crypto::Random;

#[cfg(target_os = "espidf")]
pub struct Random;

#[cfg(target_os = "espidf")]
impl crypto::Rng for Random {
    fn getrandom(buf: &mut [u8]) {
        for byte in buf {
            *byte = unsafe { esp_idf_svc::sys::random() } as u8;
        }
    }
}
