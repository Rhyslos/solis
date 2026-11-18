//! A ground's region file (`0_0.region`, decision 0116): the first byte
//! says how many chunks high the ground is (1 to 4); the rest is refused
//! or read into a region's every chunk, and what's read writes back to a
//! file that reads the same.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Some((&high, bytes)) = data.split_first() else {
        return;
    };
    let high = u32::from(high % 4) + 1;
    use ground::region;
    let Ok(chunks) = region::read(bytes, [0, 0], high) else {
        return;
    };
    let keys = region::keys([0, 0], high);
    assert_eq!(chunks.len(), keys.len());
    let mut ground = region::lone(high);
    region::load_into(&mut ground, [0, 0], bytes).expect("it read once");
    let again = region::write(&ground, [0, 0]);
    assert_eq!(region::read(&again, [0, 0], high).expect("it reads back"), chunks);
});
