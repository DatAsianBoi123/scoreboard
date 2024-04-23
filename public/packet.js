export class PacketReader {
  #index = 0;
  /**
    * @type {DataView}
    */
  #buf;

  /**
    * @param {ArrayBuffer} arrayBuffer
    */
  constructor(arrayBuffer) {
    this.#buf = new DataView(arrayBuffer);
  }

  readBool() {
    return this.readUint8() === 1;
  }

  readInt8() {
    return this.#buf.getInt8(this.#next(1));
  }

  readUint8() {
    return this.#buf.getUint8(this.#next(1));
  }

  readUint16() {
    return this.#buf.getUint16(this.#next(2), true);
  }

  readUint32() {
    return this.#buf.getUint32(this.#next(4), true);
  }

  readUint64() {
    return this.#buf.getBigUint64(this.#next(8), true);
  }

  readStringLength() {
    const len = this.readUint64();
    const utf8 = [];

    for (let i = 0; i < len; i++) {
      utf8.push(this.readUint8());
    }

    return String.fromCharCode(...utf8);
  }

  /**
    * @returns {{ duration: number, scorePoints: { name: string, category: string, points: number }[] }}
    */
  readGameInfo() {
    const gameInfo = {};
    const scores = [];

    gameInfo.duration = this.readUint16();

    while (!this.atEnd()) {
      scores.push({ name: this.readStringLength(), category: this.readStringLength(), points: this.readInt8() });
    }

    gameInfo.scorePoints = scores;

    return gameInfo;
  }

  atEnd() {
    return this.#index + 1 >= this.#buf.byteLength;
  }

  #next(amount) {
    const before = this.#index;
    this.#index += amount;
    return before;
  }
}

export class PacketWriter {
  #index = 0;
  /**
    * @type {DataView}
    */
  #buf;

  constructor(length) {
    this.#buf = new DataView(new ArrayBuffer(length));
  }

  writeUint8(data) {
    this.#buf.setUint8(this.#index, data);
    this.#index++;
  }

  writeUint64(data) {
    this.#buf.setBigUint64(this.#index, data, true);
    this.#index += 8;
  }

  get() {
    return this.#buf.buffer;
  }
}

