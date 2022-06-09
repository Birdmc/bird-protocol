use quick_error::quick_error;

quick_error! {
    #[derive(Debug)]
    pub enum CustomError {
        Error(err: Box<dyn std::error::Error + Send + Sync>) {
            display("{}", err)
            from()
            source(&**err)
        }
        String(message: String) {
            display("{}", message)
        }
        StaticStr(message: &'static str) {
            display("{}", message)
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum InputPacketBytesError {
        NoBytes(length: usize) {
            display("Length of the packet is {}", length)
        }
        Custom(err: CustomError) {
            display("Input packet caused an error {}", err)
            from()
            source(err)
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum PacketReadableError {
        InputPacketBytes(err: InputPacketBytesError) {
            display("{}", err)
            from()
            source(err)
        }
        Custom(err: CustomError) {
            display("Readable caused an error {}", err)
            from()
            source(err)
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum PacketWritableError {
        Custom(err: CustomError) {
            display("Writable caused an error {}", err)
            from()
            source(err)
        }
    }
}

pub type InputPacketBytesResult<T> = std::result::Result<T, InputPacketBytesError>;
pub type OutputPacketBytesResult = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;
pub type PacketWritableResult = std::result::Result<(), PacketWritableError>;
pub type PacketReadableResult<T> = std::result::Result<T, PacketReadableError>;

#[async_trait::async_trait]
pub trait InputPacketBytes: Send + Sync {
    async fn take_byte(&mut self) -> InputPacketBytesResult<u8>;

    async fn take_slice(&mut self, slice: &mut [u8]) -> InputPacketBytesResult<()>;

    async fn take_vec(&mut self, vec: &mut Vec<u8>) -> InputPacketBytesResult<()>;

    fn has_bytes(&self, count: usize) -> bool;

    fn remaining_bytes(&self) -> usize;
}

#[async_trait::async_trait]
pub trait OutputPacketBytes: Send + Sync {
    async fn write_byte(&mut self, byte: u8) -> OutputPacketBytesResult;

    async fn write_bytes(&mut self, slice: &[u8]) -> OutputPacketBytesResult;
}

#[async_trait::async_trait]
pub trait PacketWritable {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult;
}

#[async_trait::async_trait]
pub trait PacketReadable: Sized {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self>;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PacketState {
    Handshake,
    Status,
    Login,
    Play,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PacketSide {
    Server,
    Client,
}

pub trait Packet: PacketWritable + PacketReadable {
    fn id() -> i32;

    fn side() -> PacketSide;

    fn state() -> PacketState;

    fn protocol() -> i32;
}