#[macro_export]
macro_rules! protocol_enum {
    ($type: ty, $name: ident {
        $($var_name: ident => $var_id: expr,)*
    }) => {
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
                let value = <$type>::read(input).await?;
                match value.into() {
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

#[macro_export]
macro_rules! entity_data {
    (
        Data {
            $($id: expr => $type: ty,)*
        }
        Entities {
            $($inherit: ty, $name: ident {
                $($index: expr, $var_name: ident: $var_type: ty)*
            })*
        }
    ) => {

        #[async_trait::async_trait]
        trait EntityDataType: Readable + Writable {
            fn id() -> i32;

            async fn read_and_validate(id: i32, input: &mut impl InputByteQueue) -> Result<Self, ReadError>;
        }

        $(
            #[async_trait::async_trait]
            impl EntityDataType for $type {

                fn id() -> i32 {
                    $id
                }

                async fn read_and_validate(id: i32, input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                    match id == $id {
                        true => <$type as Readable>::read(input).await,
                        false => Err(ReadError::BadEntityDataType(id, $id)),
                    }
                }
            }
        )*


        #[async_trait::async_trait]
        trait EntityData {
            async fn read_value(&mut self, index: u8, value_type: i32, input: &mut impl InputByteQueue) -> Result<(), ReadError>;

            fn write_without_end(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError>;
        }

        #[async_trait::async_trait]
        impl EntityData for EntityNothing {
            async fn read_value(&mut self, index: u8, _value_type: i32, _input: &mut impl InputByteQueue) -> Result<(), ReadError> {
                Err(ReadError::BadEntityDataIndex(index))
            }

            fn write_without_end(&self, _output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
                Ok(())
            }
        }

        $(
            #[derive(Debug, Default)]
            pub struct $name {
                inherit: $inherit,
                $(pub $var_name: Option<$var_type>,)*
            }

            #[async_trait::async_trait]
            impl EntityData for $name {
                async fn read_value(&mut self, index: i32, value_type: i32, input: &mut impl InputByteQueue) -> Result<(), ReadError> {
                    match index {
                        $($index => {
                            self.$var_name = <$var_type as EntityDataType>::read_and_validate(value_type, input).await?;
                            Ok(())
                        },)*
                        _ => <$inherit as EntityData>::read(&mut self.inherit, index, value_type, input).await
                    }
                }

                fn write_without_end(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
                    self.inherit.write_without_end(output)?;
                    $(if let Some(val) = self.$var_name {
                        ($index as u8).write(output)?;
                        VarInt(<$var_type as EntityDataType>::id()).write(output)?;
                        self.$var_name.write(output)?;
                    })*
                }
            }

            impl Writable for $name {
                fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
                    self.write_without_end(output)?;
                    0xff_u8.write(output)
                }
            }

            #[async_trait::async_trait]
            impl Readable for $name {
                async fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                    let mut result = Self::default();
                    loop {
                        let index = u8::read(input).await?;
                        if index == 0xff {
                            break;
                        }
                        let value_type = VarInt::read(input).await?.0;
                        result.read_value(index, value_type, input)?;
                    }
                    Ok(result)
                }
            }

        )*

    }
}
