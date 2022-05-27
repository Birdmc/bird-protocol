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

        #[async_trait::async_trait]
        impl Readable for $name {
            async fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                match <$type>::read(input).await?.into() {
                    $($var_id => Ok(Self::$var_name),)*
                    _ => Err(ReadError::BadEnumValue),
                }
            }
        }
    }
}

#[macro_export]
macro_rules! bound_state_enum {
    (Server, Handshake, $($id: expr, $name: ident)*) => {
        bound_state_enum_impl!(SHPacket, $($id, $name)*);
    };
    (Server, Status, $($id: expr, $name: ident)*) => {
        bound_state_enum_impl!(SSPacket, $($id, $name)*);
    };
    (Server, Login, $($id: expr, $name: ident)*) => {
        bound_state_enum_impl!(SLPacket, $($id, $name)*);
    };
    (Server, Play, $($id: expr, $name: ident)*) => {
        bound_state_enum_impl!(SPPacket, $($id, $name)*);
    };
    (Client, Handshake, $($id: expr, $name: ident)*) => {
        bound_state_enum_impl!(CHPacket, $($id, $name)*);
    };
    (Client, Status, $($id: expr, $name: ident)*) => {
        bound_state_enum_impl!(CSPacket, $($id, $name)*);
    };
    (Client, Login, $($id: expr, $name: ident)*) => {
        bound_state_enum_impl!(CLPacket, $($id, $name)*);
    };
    (Client, Play, $($id: expr, $name: ident)*) => {
        bound_state_enum_impl!(CPPacket, $($id, $name)*);
    };
}

#[macro_export]
macro_rules! bound_packet_enum {
    ($name: ident, $hp: ident, $sp: ident, $lp: ident, $pp: ident) => {
        #[async_trait::async_trait]
        impl PacketNode for $name {
            async fn read(state: State, input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                Ok(match state {
                    State::Handshake => Self::Handshake($hp::read(input).await?),
                    State::Status => Self::Status($sp::read(input).await?),
                    State::Login => Self::Login($lp::read(input).await?),
                    State::Play => Self::Play($pp::read(input).await?),
                })
            }
        }
    };
}

#[macro_export]
macro_rules! bound_state_enum_impl {
    ($e_name: ident, $($id: expr, $name: ident)*) => {
        #[derive(Debug)]
        pub enum $e_name {
            $($name($name),)*
        }

        impl Writable for $e_name {
            fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
                match self {
                    $(Self::$name(val) => val.write(output),)*
                    o => unreachable!("unreachable. That object wasn't handled: {:?}", o)
                }
            }
        }

        #[async_trait::async_trait]
        impl Readable for $e_name {
            async fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                let num: i32 = VarInt::read(input).await?.into();
                match num {
                    $($id => Ok(Self::$name($name::read(input).await?)),)*
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
        $(
            $(

                $crate::bound_state_enum!($bound, $state, $($id, $name)*);

                $(
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

                    #[async_trait::async_trait]
                    impl Readable for $name {
                        async fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                            Ok(Self {
                                $($var_name: <$var_type as Readable>::read(input).await?,)*
                            })
                        }
                    }
                )*
            )*
        )*

        #[derive(Debug)]
        pub enum ClientPacket {
            Handshake(CHPacket),
            Status(CSPacket),
            Login(CLPacket),
            Play(CPPacket),
        }

        #[derive(Debug)]
        pub enum ServerPacket {
            Handshake(SHPacket),
            Status(SSPacket),
            Login(SLPacket),
            Play(SPPacket),
       }

        $crate::bound_packet_enum!(ClientPacket, CHPacket, CSPacket, CLPacket, CPPacket);
        $crate::bound_packet_enum!(ServerPacket, SHPacket, SSPacket, SLPacket, SPPacket);
    }
}