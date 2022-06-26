use crate::packet::*;
use crate::types::*;
use crate::packet_bytes::*;
use crate::packet_default::*;

mod primitives {
    use super::*;

    #[actix_rt::test]
    async fn numbers_test() {
        {
            let mut output = OutputPacketBytesVec::new();
            1_u8.write(&mut output).await.unwrap();
            2_u16.write(&mut output).await.unwrap();
            53232323_u32.write(&mut output).await.unwrap();
            43295438_i32.write(&mut output).await.unwrap();
            assert_eq!(output.data, vec![
                0x01,
                0x00, 0x02,
                0x03, 0x2c, 0x42, 0xc3,
                0x02, 0x94, 0xa2, 0xce,
            ])
        }
        {
            let mut input = InputPacketBytesPrepared::from(
                vec![0x03, 0xc3, 0x33, 0x21, 0x49, 0x12, 0x32]
            );
            assert_eq!(u8::read(&mut input).await.unwrap(), 0x03);
            assert_eq!(u32::read(&mut input).await.unwrap(), 0xc3332149);
            assert_eq!(u16::read(&mut input).await.unwrap(), 0x1232);
        }
    }

    #[actix_rt::test]
    async fn var_numbers_test() {
        {
            let mut output = OutputPacketBytesVec::new();
            VarInt(2097151).write(&mut output).await.unwrap();
            VarInt(2147483647).write(&mut output).await.unwrap();
            VarInt(-2147483648).write(&mut output).await.unwrap();
            VarInt(-1).write(&mut output).await.unwrap();
            VarLong(9223372036854775807).write(&mut output).await.unwrap();
            VarLong(-9223372036854775808).write(&mut output).await.unwrap();
            assert_eq!(
                output.data,
                vec![
                    0xff, 0xff, 0x7f,
                    0xff, 0xff, 0xff, 0xff, 0x07,
                    0x80, 0x80, 0x80, 0x80, 0x08,
                    0xff, 0xff, 0xff, 0xff, 0x0f,
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
                    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01,
                ]
            );
            let mut input = InputPacketBytesPrepared::from(output);
            assert_eq!(VarInt::read(&mut input).await.unwrap().0, 2097151);
            assert_eq!(VarInt::read(&mut input).await.unwrap().0, 2147483647);
            assert_eq!(VarInt::read(&mut input).await.unwrap().0, -2147483648);
            assert_eq!(VarInt::read(&mut input).await.unwrap().0, -1);
            assert_eq!(VarLong::read(&mut input).await.unwrap().0, 9223372036854775807);
            assert_eq!(VarLong::read(&mut input).await.unwrap().0, -9223372036854775808);
        }
    }

    #[actix_rt::test]
    async fn string_test() {
        {
            let mut output = OutputPacketBytesVec::new();
            "jenya705".write(&mut output).await.unwrap();
            "рашн ленг".write(&mut output).await.unwrap();
            let mut input = InputPacketBytesPrepared::from(output);
            assert_eq!(String::read(&mut input).await.unwrap(), "jenya705");
            assert_eq!(String::read(&mut input).await.unwrap(), "рашн ленг");
        }
    }

    #[actix_rt::test]
    async fn block_position_test() {
        {
            let position = BlockPosition::new(32, -32, -32195);
            let mut output = OutputPacketBytesVec::new();
            position.write(&mut output).await.unwrap();
            let mut input = InputPacketBytesPrepared::from(output);
            assert_eq!(BlockPosition::read(&mut input).await.unwrap(), position);
        }
    }
}

mod packets {
    use super::*;

    #[actix_rt::test]
    async fn handshake_test() {
        let handshake_packet = Handshaking {
            next_state: HandshakeNextState::Status,
            server_address: "localhost".into(),
            server_port: 25565,
            protocol_version: 759,
        };
        {
            let mut output = OutputPacketBytesVec::new();
            <Handshaking as PacketWritable>::write(
                handshake_packet.clone(), &mut output,
            ).await.unwrap();
            assert_eq!(output.data, vec![0, 247, 5, 9, 108, 111, 99, 97, 108, 104, 111, 115, 116, 99, 221, 1])
        }
        {
            let mut input = InputPacketBytesPrepared::from(
                vec![247, 5, 9, 108, 111, 99, 97, 108, 104, 111, 115, 116, 99, 221, 1]
            );
            assert_eq!(
                <Handshaking as PacketReadable>::read(&mut input).await.unwrap(),
                handshake_packet.clone()
            );
        }
        {
            let mut input = InputPacketBytesPrepared::from(
                vec![0, 247, 5, 9, 108, 111, 99, 97, 108, 104, 111, 115, 116, 99, 221, 1]
            );
            let packet = ClientHandshakePacket::read(&mut input).await.unwrap();
            assert_eq!(
                packet,
                ClientHandshakePacket::Handshaking(handshake_packet.clone())
            )
        }
    }
}