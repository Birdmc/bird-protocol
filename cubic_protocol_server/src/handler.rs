use std::marker::PhantomData;
use std::sync::Arc;
use cubic_protocol::packet::{InputPacketBytes, PacketReadable, PacketReadableResult, PacketState};
use crate::connection::Connection;

pub trait ConnectionHandler: Sized + Sync + Send {
    fn handle_connection(&self, connection: Arc<Connection>);

    fn handle_disconnect(&self, connection: Arc<Connection>);
}

pub trait PacketHandler<P: PacketReadable + Send + Sync>: Sized + Sync + Send {
    fn handle_packet(&self, connection: Arc<Connection>, state: &mut PacketState, packet: P);
}

#[async_trait::async_trait]
pub trait ReadHandler: Sized + Sync + Send {
    async fn handle(&self, connection: Arc<Connection>, state: &mut PacketState, input: &mut impl InputPacketBytes) -> PacketReadableResult<()>;
}

pub struct ContainerReadHandler<
    H: PacketReadable + Send + Sync,
    S: PacketReadable + Send + Sync,
    L: PacketReadable + Send + Sync,
    P: PacketReadable + Send + Sync,
    HH: PacketHandler<H>, SH: PacketHandler<S>,
    LH: PacketHandler<L>, PH: PacketHandler<P>,
> {
    handshake: HH,
    status: SH,
    login: LH,
    play: PH,
    hp: PhantomData<H>,
    sp: PhantomData<S>,
    lp: PhantomData<L>,
    pp: PhantomData<P>,
}

impl<
    H: PacketReadable + Send + Sync,
    S: PacketReadable + Send + Sync,
    L: PacketReadable + Send + Sync,
    P: PacketReadable + Send + Sync,
    HH: PacketHandler<H>, SH: PacketHandler<S>,
    LH: PacketHandler<L>, PH: PacketHandler<P>,
> ContainerReadHandler<H, S, L, P, HH, SH, LH, PH> {
    pub fn new(handshake: HH, status: SH, login: LH, play: PH) -> Self {
        Self {
            handshake,
            status,
            login,
            play,
            hp: PhantomData,
            sp: PhantomData,
            lp: PhantomData,
            pp: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<
    H: PacketReadable + Send + Sync,
    S: PacketReadable + Send + Sync,
    L: PacketReadable + Send + Sync,
    P: PacketReadable + Send + Sync,
    HH: PacketHandler<H>, SH: PacketHandler<S>,
    LH: PacketHandler<L>, PH: PacketHandler<P>,
> ReadHandler for ContainerReadHandler<H, S, L, P, HH, SH, LH, PH> {
    async fn handle(&self, connection: Arc<Connection>, state: &mut PacketState, input: &mut impl InputPacketBytes) -> PacketReadableResult<()> {
        Ok(match state {
            PacketState::Handshake =>
                self.handshake.handle_packet(connection, state, H::read(input).await?),
            PacketState::Status =>
                self.status.handle_packet(connection, state, S::read(input).await?),
            PacketState::Login =>
                self.login.handle_packet(connection, state, L::read(input).await?),
            PacketState::Play =>
                self.play.handle_packet(connection, state, P::read(input).await?),
        })
    }
}