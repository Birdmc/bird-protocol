#[macro_export]
macro_rules! packet_node {
    ($(#[$meta:meta])* $node_ident: ident => [
        $($packet_ident:ident$(<$($packet_generic:ty$(,)*)*>)*$(,)*)*
    ]) => {
        $(#[$meta])*
        pub enum $node_ident {
            $($packet_ident($packet_ident<$($($packet_generic,)*)*>),)*
        }

        $(impl From<$packet_ident<$($($packet_generic,)*)*>> for $node_ident {
            fn from(packet: $packet_ident<$($($packet_generic,)*)*>) -> Self {
                Self::$packet_ident(packet)
            }
        })*

        #[async_trait::async_trait]
        impl $crate::packet::PacketWritable for $node_ident {
            async fn write(&self, output: &mut impl $crate::packet::OutputPacketBytes) ->
                $crate::packet::PacketWritableResult {
                Ok(match self {
                    $(Self::$packet_ident(ref packet) => packet.write(output).await?,)*
                }.into())
            }
        }

        #[async_trait::async_trait]
        impl $crate::packet::PacketReadable for $node_ident {
            async fn read(input: &mut impl $crate::packet::InputPacketBytes) ->
                $crate::packet::PacketReadableResult<Self> {
                let packet_id = VarInt::read(input).await?.0;
                Ok(match packet_id {
                    $($packet_ident::<$($($packet_generic,)*)*>::ID => Self::$packet_ident(
                        $packet_ident::<$($($packet_generic,)*)*>::read(input).await?),
                    )*
                    _ => return Err(
                        $crate::packet::PacketReadableError::Custom($crate::packet::CustomError::StaticStr("Bad packet id"))
                    )
                })
            }
        }
    };
}