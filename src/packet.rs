use axum::extract::ws::Message;

use crate::{game::{GameData, BuiltinGame}, session_manager::Team};

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
                $v $({ $($f: $t),* })?,
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
        1: Score(team: Team, score_type: u8, undo: bool),
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
        4: GameData { game_type: Either<&'static BuiltinGame, GameData> },
    }
}

serverbound_packet! {
    ServerboundUserPacket {
        0: Score { score_type: u8, undo: bool },
    }
}

#[derive(Debug)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Readable, R: Readable> Readable for Either<L, R> {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        let variant: u8 = reader.read()?;
        match variant {
            0 => Some(Either::Left(L::read(reader)?)),
            1 => Some(Either::Right(R::read(reader)?)),
            _ => None,
        }
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

pub trait Readable {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized;
}

impl Readable for bool {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        reader.read_u8().map(|num| num != 0)
    }
}

impl Readable for u8 {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        reader.read_u8()
    }
}

impl Readable for i8 {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        Some(i8::from_le_bytes(reader.read_n()?))
    }
}

impl Readable for u16 {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        Some(u16::from_le_bytes(reader.read_n()?))
    }
}

impl Readable for u64 {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        Some(u64::from_le_bytes(reader.read_n()?))
    }
}

impl Readable for usize {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        Some(usize::from_le_bytes(reader.read_n()?))
    }
}

impl Readable for String {
    fn read(reader: &mut PacketReader) -> Option<Self> where Self: Sized {
        let len = reader.read()?;
        String::from_utf8(reader.read_n_slice(len)?.to_vec()).ok()
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

    pub fn read<T: Readable>(&mut self) -> Option<T> {
        T::read(self)
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        let u8 = self.buf.get(self.index).copied();
        self.index += 1;
        u8
    }

    pub fn read_n<const S: usize>(&mut self) -> Option<[u8; S]> {
        let mut bytes = [0; S];
        #[allow(clippy::needless_range_loop)]
        for i in 0..S { bytes[i] = self.read()?; }
        Some(bytes)
    }

    pub fn read_n_slice(&mut self, len: usize) -> Option<&[u8]> {
        if self.index + len >= self.buf.len() { return None };
        let slice = &self.buf[self.index..self.index + len];
        self.index += len;
        Some(slice)
    }

    pub fn has_next(&self) -> bool {
        self.index < self.buf.len()
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

