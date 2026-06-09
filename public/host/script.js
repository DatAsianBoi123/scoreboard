import { PacketReader, PacketWriter } from '../packet.js'

let rootWsUrl;
if (location.protocol === 'https:') {
  rootWsUrl = 'wss://';
} else {
  rootWsUrl = 'ws://';
}
rootWsUrl += location.host;

/**
  * @type {WebSocket}
  */
let ws;

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

const blueTeams = [];
const redTeams = [];

document.addEventListener('keydown', event => {
  if (!event.target.classList.contains('noEnter')) return;
  if (event.key != 'Enter') return;

  event.preventDefault();
});

document.getElementById('startBtn').addEventListener('click', _ => {
  if (startedTime) return;
  document.getElementById('beforeStart').style.display = 'none';
  document.getElementById('afterStart').style.display = 'block';
  startGame();
});

document.getElementById('endBtn').addEventListener('click', event => {
  if (!startedTime) return;

  if (gameEnded) {
    event.target.disabled = true;
    revealScore();
  } else {
    endGame();
  }
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

  const gameType = event.target.gameType.value;

  let data;
  if (gameType === 'builtin') {
    data = event.target.gameId.value;
  } else if (gameType === 'custom') {
    data = getGameData();
    if (!data) {
      alert('malformed form data');
      return;
    }
  } else {
    return;
  }
  host(event.target.matchNumber.value, gameType, data);
});

document.getElementById('gameTypeSelect').addEventListener('input', _ => {
  updateHostInfoForm();
});

document.getElementById('newRowBtn').addEventListener('click', _ => newRow());

document.getElementById('copyGameDataBtn').addEventListener('click', async _ => {
  const gameData = getGameData();
  if (!gameData) {
    alert('malformed form data');
    return;
  }
  const writer = new PacketWriter(gameData[1]);
  writer.writeGameData(gameData[0]);

  let binary = '';
  const bytes = new Uint8Array(writer.get());
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }

  await navigator.clipboard.writeText(btoa(binary));
});

document.getElementById('importGameDataBtn').addEventListener('click', async _ => {
  const code = (await navigator.clipboard.readText()).trim();
  if (!code) return;
  const data = Uint8Array.fromBase64(code);
  const gameInfo = new PacketReader(data.buffer).readGameInfo();

  document.getElementById('durationMinInput').value = Math.floor(gameInfo.duration / 60);
  document.getElementById('durationSecsInput').value = gameInfo.duration % 60;
  const tableBody = document.querySelector('#scoreTable tbody');
  tableBody.replaceChildren();
  for (const scorePoint of gameInfo.scorePoints) {
    newRow(scorePoint.name, scorePoint.category, scorePoint.points);
  }
});

document.getElementById('blueAddTeam').addEventListener('submit', event => {
  event.preventDefault();

  addTeamName('blue', event.target);
});

document.getElementById('redAddTeam').addEventListener('submit', event => {
  event.preventDefault();

  addTeamName('red', event.target);
});

function addTeamName(team, form) {
  const teams = document.getElementById(`${team}Teams`);
  const nameInput = form.teamName;

  const name = nameInput.value.trim();
  if (name.length == 0) return;

  nameInput.value = "";

  const li = document.createElement('li');
  li.appendChild(document.createTextNode(`${name} `));

  const deleteInput = document.createElement('input');
  deleteInput.type = 'button';
  deleteInput.value = '-';
  deleteInput.addEventListener('click', event => {
    event.target.parentElement.remove();
  });
  li.appendChild(deleteInput);

  teams.appendChild(li);

  if (team === 'blue') blueTeams.push(name);
  else if (team === 'red') redTeams.push(name);
}

updateHostInfoForm();

fetch('/api/builtin-games')
  .then(res => res.json())
  .then(retrieveBuiltinGames);

function updateHostInfoForm() {
  const gameType = document.getElementById('gameTypeSelect').value;
  const builtinGameType = document.getElementById('builtinGameType');
  const customGameType = document.getElementById('customGameType');

  builtinGameType.style.display = 'none';
  customGameType.style.display = 'none';
  if (gameType === 'builtin') builtinGameType.style.display = 'block';
  else if (gameType === 'custom') customGameType.style.display = 'block';
}

function getGameData() {
  const form = document.getElementById('hostInfoForm');
  const duration = (form.durationMin.value ?? 0) * 60 + parseInt(form.durationSecs.value ?? 0);
  const scorePoints = [];
  let length = 2;

  for (const row of document.getElementsByClassName('scoreTableRow')) {
    const name = row.getElementsByClassName('name')[0].value;
    const category = row.getElementsByClassName('category')[0].value;
    const points = parseInt(row.getElementsByClassName('points')[0].value);

    if (!name || !category || !points) return null;

    scorePoints.push({ name, category, points });
    const encoder = new TextEncoder();
    length += 17 + encoder.encode(name).length + encoder.encode(category).length;
  }

  return [{ duration, scorePoints }, length];
}

/**
 * @param {string?} name
 * @param {string?} category
 * @param {number?} points
 */
function newRow(name, category, points) {
  const scoreTable = document.querySelector('#scoreTable tbody');

  const row = scoreTable.insertRow();
  row.classList.add('scoreTableRow');

  const editCell = row.insertCell();
  const moveUp = document.createElement('input');
  moveUp.type = 'button';
  moveUp.value = '^';
  moveUp.title = 'Move Row Up';
  moveUp.classList.add('editInput');
  moveUp.addEventListener('click', _ => {
    const above = row.previousElementSibling;
    if (above) {
      row.insertAdjacentElement('afterend', above);
    }
  });
  const moveDown = document.createElement('input');
  moveDown.type = 'button';
  moveDown.value = 'v';
  moveDown.title = 'Move Row Down';
  moveDown.classList.add('editInput');
  moveDown.addEventListener('click', _ => {
    const above = row.nextElementSibling;
    if (above) {
      row.insertAdjacentElement('beforebegin', above);
    }
  });
  const remove = document.createElement('input');
  remove.type = 'button';
  remove.value = '-';
  remove.title = 'Remove Row';
  remove.classList.add('editInput');
  remove.addEventListener('click', _ => {
    row.remove();
  });
  editCell.appendChild(moveUp);
  editCell.appendChild(moveDown);
  editCell.appendChild(remove);
  for (let i = 0; i < 3; i++) {
    const td = row.insertCell();

    const input = document.createElement('input');
    input.type = i === 2 ? 'number' : 'text';
    input.classList.add('noEnter');

    if (i === 0) {
      input.classList.add('name');
      input.value = name ?? '';
    } else if (i === 2) {
      input.classList.add('points');
      input.value = points ?? 0;
    } else if (i === 1) {
      input.classList.add('category');
      input.value = category ?? '';

      const wrapper = document.createElement('div');
      wrapper.classList.add('dropdownWrapper');

      input.addEventListener('focusin', _ => {
        dropdown.replaceChildren(generateCategoryList(input));
        dropdown.classList.add('focus');
      });
      input.addEventListener('focusout', _ => {
        dropdown.classList.remove('focus');
      });

      const dropdown = document.createElement('div');
      dropdown.classList.add('textDropdown');

      wrapper.appendChild(input);
      wrapper.appendChild(dropdown);

      td.appendChild(wrapper);
    }

    if (i !== 1) td.appendChild(input);
  }
}

function generateCategoryList(input) {
  const ul = document.createElement('ul');

  const categories = new Set();

  for (const category of document.querySelectorAll('#scoreTable input.category')) {
    const value = category.value;
    if (value.trim() === '') continue;
    categories.add(value);
  }

  for (const category of categories) {
    const li = document.createElement('li');
    li.innerText = category;
    li.addEventListener('mousedown', event => {
      event.preventDefault();
      input.value = event.target.innerText;
    });

    ul.appendChild(li);
  }

  return ul;
}

/**
  * @param {{ name: string, data: any }[]} builtinGames
  */
function retrieveBuiltinGames(builtinGames) {
  document.getElementById('loadingDiv').style.display = 'none';

  const gameTypeSelect = document.getElementById('gameIdSelect');

  for (let i = 0; i < builtinGames.length; i++) {
    const option = document.createElement('option');
    option.innerText = builtinGames[i].name;
    option.value = i;
    gameTypeSelect.appendChild(option);
  }

  document.getElementById('prehost').style.display = 'block';
}

/**
  * @param {number} matchNumber
  * @param {'builtin' | 'custom'} gameType
  */
function host(matchNumber, gameType, data) {
  document.getElementById('prehost').style.display = 'none';
  document.getElementById('loadingDiv').style.display = 'block';

  ws = new WebSocket(`${rootWsUrl}/ws/host`);

  ws.addEventListener('open', _ => {
    console.log('connected to ws');
    console.log(gameType);
    console.log(data);

    let blueNameSize = blueTeams.length * 8;
    for (const name of blueTeams) {
      blueNameSize += name.length;
    }

    let redNameSize = redTeams.length * 8;
    for (const name of redTeams) {
      redNameSize += name.length;
    }
    const nameSize = 16 + blueNameSize + redNameSize;

    let writer;
    if (gameType === 'builtin') {
      writer = new PacketWriter(nameSize + 12);
    } else if (gameType === 'custom') {
      writer = new PacketWriter(nameSize + 4 + data[1]);
    }

    writer.writeUint8(4);
    writer.writeUint16(matchNumber);
    writer.writeStringArray(blueTeams);
    writer.writeStringArray(redTeams);

    if (gameType === 'builtin') {
      writer.writeUint8(0);
      writer.writeUint64(BigInt(data));
    } else if (gameType === 'custom') {
      writer.writeUint8(1);
      writer.writeGameData(data[0]);
    }
    ws.send(writer.get());
  });

  ws.addEventListener('close', _ => {
    disconnect();

    console.log('connection closed');
  });

  ws.addEventListener('error', _ => {
    disconnect();

    console.log('got error');
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
        const undo = reader.readBool();

        score(team, scoreId, undo);
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
        clearInterval(updateTimeIntervalId);
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
  document.getElementById('endBtn').innerText = 'Reveal Score';
  document.getElementById('pauseBtn').disabled = true;
}

function revealScore() {
  const writer = new PacketWriter(1);
  writer.writeUint8(5);
  ws.send(writer.get());

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
  * @param {number} scoreId
  * @param {boolean} undo
  */
function score(team, scoreId, undo) {
  const scorePoints = gameInfo.scorePoints[scoreId];
  const points = (undo ? -1 : 1) * scorePoints.points;
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

  addScoreLog(teamString, scorePoints, undo);

  const scoreBeginning = undo ? `${teamString} team undo scored` : `${teamString} team scored`;
  console.log(`${scoreBeginning} (${points < 0 ? '-' : '+'}${Math.abs(points)})`);
}

function addScoreLog(team, scorePoints, undo) {
  const points = (undo ? -1 : 1) * scorePoints.points;

  const row = document.createElement('tr');
  row.classList.add(team);

  const teamCell = document.createElement('td');
  teamCell.innerText = team;

  const scoredCell = document.createElement('td');
  scoredCell.innerText = (undo ? 'UNDO ' : '') + scorePoints.name;

  const pointsCell = document.createElement('td');
  pointsCell.innerText = `${points < 0 ? '-' : '+'}${Math.abs(points)}`;

  const timestampCell = document.createElement('td');
  timestampCell.innerText = formatTime(getCurrentTimeLeft());

  row.appendChild(teamCell);
  row.appendChild(scoredCell);
  row.appendChild(pointsCell);
  row.appendChild(timestampCell);

  document.querySelector('#scoreHistory tbody').appendChild(row);
}

function getCurrentTimeLeft() {
  if (gameEnded) return 0;

  const now = gamePaused ? pauseStarted : Date.now();
  return gameInfo.duration * 1000 - (now - (startedTime + pausedTime));
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
