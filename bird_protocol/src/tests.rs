use crate::packet::*;
use crate::packet_types::*;

#[test]
fn number_tests() {
    {
        let mut read = PacketRead::new(&[0x32, 0x15, 0x44, 0x32, 0x33, 0x55, 0x18]);
        assert_eq!(u8::read(&mut read).unwrap(), 0x32u8);
        assert_eq!(u16::read(&mut read).unwrap(), 0x1544u16);
        assert_eq!(u32::read(&mut read).unwrap(), 0x32335518u32);
    }
    {
        let mut write = Vec::new();
        0x32u8.write(&mut write).unwrap();
        0x5555u16.write(&mut write).unwrap();
        0x03255213i32.write(&mut write).unwrap();
        assert_eq!(write, &[0x32, 0x55, 0x55, 0x03, 0x25, 0x52, 0x13]);
    }
}

#[test]
fn string_tests() {
    {
        let mut write = Vec::new();
        "jenya705 is good boy".write(&mut write).unwrap();
        "женя705 ис гуд бой".write(&mut write).unwrap();
        let mut read = PacketRead::new(write.as_slice());
        assert_eq!(<&str>::read(&mut read).unwrap(), "jenya705 is good boy");
        assert_eq!(String::read(&mut read).unwrap(), "женя705 ис гуд бой");
    }
}