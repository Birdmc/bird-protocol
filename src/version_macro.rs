#[macro_export]
macro_rules! protocol_enum {
    ($type: ty, $name: ident {
        $($var_name: ident => $var_id: expr,)*
    }) => {
        use cubic_protocol::version::*;
        use cubic_protocol::protocol::*;
        use cubic_protocol::bytes::*;

        #[derive(PartialEq, Copy, Clone, Debug)]
        pub enum $name {
            $($var_name,)*
        }

        impl Writable for $name {
            fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
                match self {
                    $(Self::$var_name => <$type>::from($var_id).write(output),)*
                }
            }
        }

        impl Readable for $name {
            fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                match <$type>::read(input)?.into() {
                    $($var_id => Ok(Self::$var_name),)*
                    _ => Err(ReadError::BadEnumValue),
                }
            }
        }
    }
}

#[macro_export]
macro_rules! protocol_packets {
    ($protocol: expr, $protocol_name: expr => {$(
        $state: ident {$(
           $bound: ident {$(
               $id: expr, $name: ident {
                   $($var_name: ident: $var_type: ty,)*
               }
           )*}
        )*}
    )*}
    ) => {
        use cubic_protocol::version::*;
        use cubic_protocol::protocol::*;
        use cubic_protocol::bytes::*;
        use cubic_chat::component::*;
        use cubic_chat::identifier::*;
        use uuid::Uuid;
        $($($(
            #[derive(Debug)]
            pub struct $name {
                $(pub $var_name: $var_type,)*
            }

            impl Packet for $name {
                fn id() -> i32 {
                    $id
                }

                fn bound() -> Bound {
                    Bound::$bound
                }

                fn state() -> State {
                    State::$state
                }

                fn protocol() -> i32 {
                    $protocol
                }
            }

            impl Writable for $name {
                fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
                    VarInt(Self::id()).write(output)?;
                    $(self.$var_name.write(output)?;)*
                    Ok(())
                }
            }

            impl Readable for $name {
                fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                    Ok(Self {
                        $($var_name: <$var_type as Readable>::read(input)?,)*
                    })
                }
            }
        )*)*)*
    }
}