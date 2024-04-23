import { PacketReader, PacketWriter } from '../packet.js'

let rootWsUrl;
if (location.protocol === 'https:') {
  rootWsUrl = 'wss://';
} else {
  rootWsUrl = 'ws://';
}
rootWsUrl += location.host;

/**
  * @type {number}
  */
let id;

/**
  * @type {{ duration: number, scorePoints: { name: string, category: string, points: number }[] }}
  */
let gameInfo;

let gameEnded = false;
let startedTime;

let gamePaused = false;
let pauseStarted;
let pausedTime = 0;

const bluePointsSpan = document.getElementById('bluePoints');
const redPointsSpan = document.getElementById('redPoints');

const BLUE_ID = 0;
const RED_ID = 1;

let bluePoints = 0;
let redPoints = 0;

document.getElementById('startBtn').addEventListener('click', _ => {
  if (startedTime) return;
  document.getElementById('beforeStart').style.display = 'none';
  document.getElementById('afterStart').style.display = 'block';
  startGame();
});

document.getElementById('endBtn').addEventListener('click', event => {
  if (!startedTime) return;
  event.target.disabled = true;
  document.getElementById('pauseBtn').disabled = true;
  endGame();
});

document.getElementById('pauseBtn').addEventListener('click', event => {
  if (!startedTime) return;
  if (gamePaused) {
    unpauseGame();
    event.target.innerText = 'Pause';
  } else {
    pauseGame();
    event.target.innerText = 'Unpause';
  }
});

document.getElementById('hostInfoForm').addEventListener('submit', event => {
  event.preventDefault();

  host(event.target.gameType.value);
});

fetch(`/api/builtin-games`)
  .then(res => res.json())
  .then(retrieveBuiltinGames);

/**
  * @param {{ name: string, data: any }[]} builtinGames 
  */
function retrieveBuiltinGames(builtinGames) {
  document.getElementById('loadingDiv').style.display = 'none';

  const gameTypeSelect = document.getElementById('gameTypeSelect');

  for (let i = 0; i < builtinGames.length; i++) {
    const option = document.createElement('option');
    option.innerText = builtinGames[i].name;
    option.value = i;
    gameTypeSelect.appendChild(option);
  }

  document.getElementById('prehost').style.display = 'block';
}

function host(gameType) {
  document.getElementById('prehost').style.display = 'none';

  const ws = new WebSocket(`${rootWsUrl}/ws/host/${gameType}`);

  ws.addEventListener('open', _ => {
    console.log('connected to ws');
  });

  ws.addEventListener('close', _ => {
    disconnect();

    console.log('connection closed');
  });

  ws.addEventListener('error', event => {
    disconnect();

    console.log(`got error: ${event}`);
  });

  ws.addEventListener('message', async event => {
    const reader = new PacketReader(await event.data.arrayBuffer());
    switch (reader.readUint8()) {
      // SessionInfo
      case 0: {
        console.log('session info');

        id = reader.readUint32();
        gameInfo = reader.readGameInfo();

        console.log(`id: ${id}`);
        console.log(`duration: ${gameInfo.duration}s`);
        console.table(gameInfo.scorePoints);

        init();

        break;
      };

      // ScoreType
      case 1: {
        console.log('score type');

        const team = reader.readUint8();
        const scoreId = reader.readUint8();

        score(team, scoreId);
      }
    }
  });
}

function init() {
  document.getElementById('loadingDiv').style.display = 'none';
  document.getElementById('main').style.display = 'block';

  document.getElementById('hostId').innerText = id.toString(36);

  document.getElementById('viewLink').href = `/view?id=${id.toString(36)}`;

  startUpdateTimeInterval();
}

function startUpdateTimeInterval() {
  const updateTimeIntervalId = setInterval(() => {
    if (gamePaused) return;

    const timeLeftH1 = document.getElementById('timeLeft');

    if (gameEnded) {
      timeLeftH1.innerText = 'GAME ENDED';
      clearInterval(updateTimeIntervalId);
      return;
    } 
    
    let time;
    if (!startedTime) {
      time = formatTime(gameInfo.duration * 1000);
    } else {
      const timeLeft = getCurrentTimeLeft();
      if (timeLeft <= 0) {
        endGame();
        return;
      }
      time = formatTime(timeLeft);
    }

    if (timeLeftH1.innerText !== time) timeLeftH1.innerText = time;
  }, 1);
}

function startGame() {
  startedTime = Date.now();

  const writer = new PacketWriter(9);
  writer.writeUint8(0);
  writer.writeUint64(BigInt(startedTime));
  ws.send(writer.get());
}

function endGame() {
  const writer = new PacketWriter(1);
  writer.writeUint8(1);
  ws.send(writer.get());

  gameEnded = true;
  startedTime = null;
}

function pauseGame() {
  const writer = new PacketWriter(1);
  writer.writeUint8(2);
  ws.send(writer.get());

  pauseStarted = Date.now();
  gamePaused = true;
}

function unpauseGame() {
  const timePaused = Date.now() - pauseStarted;

  const writer = new PacketWriter(9);
  writer.writeUint8(3);
  writer.writeUint64(BigInt(timePaused));
  ws.send(writer.get());

  pauseStarted = null;
  gamePaused = false;
  pausedTime += timePaused;
}

function disconnect() {
  document.getElementById('loadingDiv').style.display = 'none';
  document.getElementById('main').style.display = 'none';
  document.getElementById('disconnectDiv').style.display = 'block';
}

/**
  * @param {0 | 1} team 
  * @param {bigint} scoreId 
  */
function score(team, scoreId) {
  const scorePoints = gameInfo.scorePoints[parseInt(scoreId)];
  const points = scorePoints.points;
  let teamString;
  if (team === BLUE_ID) {
    bluePoints += points;
    bluePointsSpan.innerText = bluePoints;
    teamString = 'blue';
  } else if (team === RED_ID) {
    redPoints += points;
    redPointsSpan.innerText = redPoints;
    teamString = 'red';
  }

  addScoreLog(teamString, scorePoints);

  console.log(`${teamString} team scored ${scorePoints.name} (${points < 0 ? '-' : '+'}${Math.abs(points)})`);
}

function addScoreLog(team, scorePoints) {
  const points = scorePoints.points;

  const row = document.createElement('tr');
  row.classList.add(team);

  const teamCell = document.createElement('td');
  teamCell.innerText = team;

  const scoredCell = document.createElement('td');
  scoredCell.innerText = scorePoints.name;

  const pointsCell = document.createElement('td');
  pointsCell.innerText = `${points < 0 ? '-' : '+'}${Math.abs(points)}`;

  const timestampCell = document.createElement('td');
  timestampCell.innerText = formatTime(getCurrentTimeLeft());

  row.appendChild(teamCell);
  row.appendChild(scoredCell);
  row.appendChild(pointsCell);
  row.appendChild(timestampCell);

  document.getElementById('scoreHistory').appendChild(row);
}

function getCurrentTimeLeft() {
  return gameInfo.duration * 1000 - (Date.now() - (startedTime + pausedTime));
}

function formatTime(time) {
  const minutes = Math.floor(time / 1000 / 60);
  const seconds = Math.floor(time / 1000 % 60);

  return `${minutes}:${leftPad(seconds.toString(), '0', 2)}`
}

/**
  * @param {string} str 
  * @param {string} padString 
  * @param {number} size 
  */
function leftPad(str, padString, size) {
  let padded = '';

  while (padded.length + str.length < size) {
    padded += padString;
  }

  return padded + str;
}

