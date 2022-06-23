#[macro_export]
macro_rules! packet_enum {
    ($(#[$meta:meta])* $name: ident, $type: ty => {
        $($var_name: ident = $var_value: expr$(,)*)*
    }) => {
        $(#[$meta])*
        pub enum $name {
            $($var_name,)*
        }

        #[async_trait::async_trait]
        impl $crate::packet::PacketReadable for $name {
            async fn read(input: &mut impl $crate::packet::InputPacketBytes) ->
                $crate::packet::PacketReadableResult<Self> {
                let value = <$type>::read(input).await?.into();
                Ok(match value {
                    $($var_value => Self::$var_name,)*
                    #[allow(unreachable_code)]
                    _ => return Err($crate::packet::CustomError::StaticStr("Bad enum value").into())
                })
            }
        }

        #[async_trait::async_trait]
        impl $crate::packet::PacketWritable for $name {
            async fn write(self, output: &mut impl $crate::packet::OutputPacketBytes) ->
                $crate::packet::PacketWritableResult {
                <$type>::from(match self {
                    $(Self::$var_name => $var_value,)*
                }).write(output).await
            }
        }
    }
}