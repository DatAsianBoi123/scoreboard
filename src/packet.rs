use axum::extract::ws::Message;

use crate::{game::GameData, session_manager::Team};

macro_rules! clientbound_packet {
    ($n: ident { $($i: literal : $v: ident $(($($m: ident: $t: ty),+))? ),* $(,)?}) => {
        #[derive(Debug)]
        pub enum $n {
            $($v ($( $($t,)+ )?),)*
        }

        impl ClientboundPacket for $n {
            fn id(&self) -> u8 {
                match self {
                    $(
                        Self::$v(..) => $i,
                    )*
                }
            }

            fn write(self, writer: &mut PacketWriter) {
                match self {
                    $(
                        Self::$v( $($($m,)+)? ) => {
                            $($(
                                writer.write($m);
                            )?)*
                        }
                    )*
                }
            }
        }
    };
}

macro_rules! serverbound_packet {
    ($n: ident { $($i: literal : $v: ident $({ $($f: ident : $t: ty),* $(,)? })?),* $(,)? }) => {
        #[derive(Debug)]
        pub enum $n {
            $(
                $v $({ $($f: $t)* })?,
            )*
        }

        impl ServerboundPacket for $n {
            fn read(reader: &mut PacketReader) -> Option<Self> {
                let id: u8 = reader.read()?;

                match id {
                    $(
                        $i => {
                            $($(
                                let $f = reader.read()?;
                            )*)?
                            Some(Self::$v $({ $($f),* })?)
                        }
                    )*
                    _ => None,
                }
            }
        }
    };
}

clientbound_packet! {
    ClientboundHostPacket {
        0: SessionInfo(id: u32, game_data: GameData),
        1: Score(team: Team, score_type: u8),
    }
}

clientbound_packet! {
    ClientboundUserPacket {
        0: SessionInfo(started: bool, game_data: GameData),
        1: StartGame,
        2: EndGame,
    }
}

serverbound_packet! {
    ServerboundHostPacket {
        0: StartGame { time_started: u64 },
        1: EndGame,
        2: PauseGame,
        3: UnpauseGame { time_paused: u64 },
    }
}

serverbound_packet! {
    ServerboundUserPacket {
        0: Score { score_type: u8 },
    }
}

pub trait IntoMessage {
    fn into_message(self) -> Message;
}

impl<T> IntoMessage for T
where T: ClientboundPacket
{
    fn into_message(self) -> Message {
        let mut writer = PacketWriter::new();
        writer.write_u8(self.id());
        self.write(&mut writer);
        Message::Binary(writer.get())
    }
}

pub trait FromMessage {
    fn from_message(message: Message) -> Option<Self> where Self: Sized;
}

impl<T> FromMessage for T
where T: ServerboundPacket
{
    fn from_message(message: Message) -> Option<Self> {
        if let Message::Binary(binary) = message {
            T::read(&mut PacketReader::new(binary.into()))
        } else {
            None
        }
    }
}

pub trait ClientboundPacket {
    fn id(&self) -> u8;

    fn write(self, writer: &mut PacketWriter);
}

pub trait ServerboundPacket {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized;
}

pub trait Readable<const S: usize> {
    fn read(bytes: [u8; S]) -> Option<Self> where Self: Sized;
}

impl Readable<1> for u8 {
    fn read(bytes: [u8; 1]) -> Option<Self> where Self: Sized {
        Some(bytes[0])
    }
}

impl Readable<8> for u64 {
    fn read(bytes: [u8; 8]) -> Option<Self> where Self: Sized {
        Some(u64::from_le_bytes(bytes))
    }
}

pub trait Writable {
    fn write(self, writer: &mut PacketWriter);
}

impl Writable for bool {
    fn write(self, writer: &mut PacketWriter) {
        writer.write(self as u8);
    }
}

impl Writable for i8 {
    fn write(self, writer: &mut PacketWriter) {
        writer.write_all(self.to_le_bytes());
    }
}

impl Writable for u8 {
    fn write(self, writer: &mut PacketWriter) {
        writer.write_u8(self);
    }
}

impl Writable for u16 {
    fn write(self, writer: &mut PacketWriter) {
        writer.write_all(self.to_le_bytes());
    }
}

impl Writable for u32 {
    fn write(self, writer: &mut PacketWriter) {
        writer.write_all(self.to_le_bytes());
    }
}

impl Writable for usize {
    fn write(self, writer: &mut PacketWriter) {
        writer.write_all(self.to_le_bytes());
    }
}

impl Writable for String {
    fn write(self, writer: &mut PacketWriter) {
        writer.write(self.len());
        writer.write_all(self.into_bytes());
    }
}

pub struct PacketReader {
    index: usize,
    buf: Box<[u8]>,
}

impl PacketReader {
    pub fn new(buf: Box<[u8]>) -> Self {
        Self { index: 0, buf }
    }

    pub fn read<T: Readable<S>, const S: usize>(&mut self) -> Option<T> {
        let mut bytes = [0; S];
        #[allow(clippy::needless_range_loop)]
        for i in 0..S {
            bytes[i] = self.buf.get(self.index).copied()?;
            self.index += 1;
        }
        T::read(bytes)
    }
}

pub struct PacketWriter {
    bytes: Vec<u8>,
}

impl PacketWriter {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn write<W: Writable>(&mut self, data: W) {
        data.write(self);
    }

    pub fn write_all(&mut self, data: impl IntoIterator<Item = impl Writable>) {
        data.into_iter().for_each(|writable| self.write(writable));
    }

    pub fn write_u8(&mut self, data: u8) {
        self.bytes.push(data);
    }

    pub fn get(self) -> Vec<u8> {
        self.bytes
    }
}

