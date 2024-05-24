import { PacketReader, PacketWriter } from "../packet.js";

let rootWsUrl;
if (location.protocol === 'https:') {
  rootWsUrl = 'wss://';
} else {
  rootWsUrl = 'ws://';
}
rootWsUrl += location.host;

const query = new URLSearchParams(location.search);
const team = query.get('team');
const id = parseInt(query.get('sessionId'), 36);

const ws = new WebSocket(`${rootWsUrl}/ws/join/${id}/${team}`);

const SCORES_DIV = document.getElementById('scores');

let started = false;
/**
  * @type {{ duration: number, scorePoints: { name: string, category: string, points: number }[] }}
  */
let gameInfo;

ws.addEventListener('open', _ => {
  console.log('opened websocket');
});

ws.addEventListener('close', _ => {
  console.log('closed websocket');

  disconnect();
});

ws.addEventListener('message', async event => {
  const reader = new PacketReader(await event.data.arrayBuffer());
  switch (reader.readUint8()) {
    // SessionInfo
    case 0: {
      console.log('session info')
      started = reader.readBool();
      gameInfo = reader.readGameInfo();

      console.log(`started? ${started}`);
      console.log(`duration: ${gameInfo.duration}`);
      console.table(gameInfo.scorePoints);

      init();

      if (started) startGame();

      break;
    };

    // StartGame
    case 1: {
      started = true;

      startGame();

      break;
    };

    // EndGame
    case 2: {
      started = false;

      endGame();

      break;
    };
  }
});

function init() {
  document.getElementById('main').style.display = 'block';
  document.getElementById('loadingDiv').style.display = 'none';
  document.getElementById('team').innerText = team;

  document.getElementById('mainColorBar').classList.add(team);

  for (let i = 0; i < gameInfo.scorePoints.length; i++) {
    const scorePoint = gameInfo.scorePoints[i];
    const button = document.createElement('button');
    const points = scorePoint.points;
    button.disabled = true;
    button.classList.add('score');
    button.innerText = scorePoint.name;
    button.addEventListener('click', () => {
      if (started) score(i);
    });

    const subtext = document.createElement('div');
    subtext.classList.add('subtext');
    subtext.innerText = `${points < 0 ? '-' : '+'}${Math.abs(points)}`;

    button.appendChild(subtext);

    SCORES_DIV.appendChild(button);
  }
}

function startGame() {
  for (const scoreButton of SCORES_DIV.children) {
    scoreButton.disabled = false;
  }
}

function endGame() {
  for (const scoreButton of SCORES_DIV.children) {
    scoreButton.disabled = true;
  }
}

function disconnect() {
  document.getElementById('main').style.display = 'none';
  document.getElementById('loadingDiv').style.display = 'none';
  document.getElementById('disconnectDiv').style.display = 'block';
}

function score(id) {
  const writer = new PacketWriter(3);
  writer.writeUint8(0);
  writer.writeUint8(id);
  writer.writeBool(document.getElementById('undoCheckbox').checked);
  ws.send(writer.get());
}

