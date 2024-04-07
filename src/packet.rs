use std::mem::size_of;

use axum::extract::ws::Message;

use crate::{session_manager::Team, game::{GameData, ScorePoint}};

macro_rules! clientbound_packet {
    ($n: ident @ $wn: ident { $($i: literal : $v: ident $(($($m: ident: $t: ty),+))? $( => $w: block),* $(,)? )*}) => {
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
                let $wn = writer;

                match self {
                    $(
                        Self::$v( $($($m,)+)? ) => {
                            $($(
                                let $m = $m;
                            )?)*
                            $(
                                $w;
                            )?
                        }
                    )*
                }
            }
        }
    };
}

macro_rules! serverbound_packet {
    ($n: ident @ $r: ident { $($i: literal : $v: ident $({ $($f: ident : $t: ty = $b: block),* $(,)? })?),* $(,)? }) => {
        #[derive(Debug)]
        pub enum $n {
            $(
                $v $({ $($f: $t)* })?,
            )*
        }

        impl ServerboundPacket for $n {
            fn read(reader: &mut PacketReader) -> Option<Self> {
                let id = reader.read_u8()?;
                let $r = reader;

                match id {
                    $(
                        $i => {
                            $($(
                                let $f = $b?;
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
    ClientboundHostPacket @ writer {
        0: SessionInfo(id: u32, game_data: GameData) => {
            writer.write_u32_le(id);
            write_game_data(writer, game_data);
        },
        1: Score(team: Team, score_type: u8) => {
            writer.write_u8(team as u8);
            writer.write_u8(score_type);
        },
    }
}

clientbound_packet! {
    ClientboundUserPacket @ writer {
        0: SessionInfo(started: bool, game_data: GameData) => {
            writer.write_bool(started);
            write_game_data(writer, game_data);
        },
        1: StartGame,
        2: EndGame,
    }
}

serverbound_packet! {
    ServerboundHostPacket @ reader {
        0: StartGame {
            time_started: u64 = {
                reader.read_u64_le()
            },
        },
        1: EndGame,
    }
}

serverbound_packet! {
    ServerboundUserPacket @ reader {
        0: Score {
            score_type: u8 = {
                reader.read_u8()
            },
        },
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

pub fn write_game_data(writer: &mut PacketWriter, game_data: GameData) {
    writer.write_u16_le(game_data.duration.secs);
    for ScorePoint { name, category, points } in Vec::from(game_data.score_points) {
        writer.write_string_len(name);
        writer.write_string_len(category);
        writer.write_i8_le(points);
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

    pub fn read_u8(&mut self) -> Option<u8> {
        let res = self.buf.get(self.index);
        self.index += 1;
        res.copied()
    }

    pub fn read_u64_le(&mut self) -> Option<u64> {
        // this is so ugly
        Some(u64::from_le_bytes([
                self.read_u8()?,
                self.read_u8()?,
                self.read_u8()?,
                self.read_u8()?,
                self.read_u8()?,
                self.read_u8()?,
                self.read_u8()?,
                self.read_u8()?,
        ]))
    }

    pub fn read_usize_le(&mut self) -> Option<usize> {
        let mut bytes: [u8; size_of::<usize>()] = Default::default();
        #[allow(clippy::needless_range_loop)]
        for i in 0..bytes.len() {
            bytes[i] = self.read_u8()?;
        }
        Some(usize::from_le_bytes(bytes))
    }
}

pub struct PacketWriter {
    bytes: Vec<u8>,
}

impl PacketWriter {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn write_all(&mut self, data: impl IntoIterator<Item = u8>) {
        data.into_iter().for_each(|byte| self.write_u8(byte));
    }

    pub fn write_bool(&mut self, data: bool) {
        self.write_u8(data as u8)
    }

    pub fn write_i8_le(&mut self, data: i8) {
        self.bytes.push(data.to_le_bytes()[0]);
    }

    pub fn write_u8(&mut self, data: u8) {
        self.bytes.push(data);
    }

    pub fn write_u16_le(&mut self, data: u16) {
        self.write_all(data.to_le_bytes())
    }

    pub fn write_u32_le(&mut self, data: u32) {
        self.write_all(data.to_le_bytes())
    }

    pub fn write_usize_le(&mut self, data: usize) {
        self.write_all(data.to_le_bytes())
    }

    pub fn write_string_len(&mut self, data: String) {
        self.write_usize_le(data.len());
        self.write_all(data.into_bytes());
    }

    pub fn get(self) -> Vec<u8> {
        self.bytes
    }
}

