let rootWsUrl;
if (location.protocol === 'https:') {
  rootWsUrl = 'wss://';
} else {
  rootWsUrl = 'ws://';
}
rootWsUrl += location.host;

const searchParams = new URLSearchParams(window.location.search);
const ws = new WebSocket(`${rootWsUrl}/ws/host/${searchParams.get('gameType')}`);

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
  endGame();
});

ws.addEventListener('open', _ => {
  console.log('connected to ws');
});

ws.addEventListener('close', _ => {
  disconnect();

  console.log('connection closed');
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

function init() {
  document.getElementById('loadingDiv').style.display = 'none';
  document.getElementById('main').style.display = 'block';

  document.getElementById('hostId').innerText = id.toString(36);

  document.getElementById('viewLink').href = `/view?id=${id.toString(36)}`;

  startUpdateTimeInterval();
}

function startUpdateTimeInterval() {
  const updateTimeIntervalId = setInterval(() => {
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

  let writer = new PacketWriter(9);
  writer.writeUint8(0);
  writer.writeUint64(BigInt(startedTime));
  ws.send(writer.get());
}

function endGame() {
  let writer = new PacketWriter(1);
  writer.writeUint8(1);
  ws.send(writer.get());

  gameEnded = true;
  startedTime = null;
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
  return gameInfo.duration * 1000 - (Date.now() - startedTime);
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

