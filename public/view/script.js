const query = new URLSearchParams(window.location.search);

const eventSource = new EventSource(`/sse/view/${parseInt(query.get('id'), 36)}`);

let startedTime;
let gameEnded = false;

let timePaused = 0;
let gamePaused = false;
/**
  * @type {{ name: string, category: string, points: number }[]}
  */
let scorePoints;

const points = {
  blue: 0,
  red: 0,
};

eventSource.addEventListener('message', event => {
  /**
    * @type {{ type: 'session_info' | 'score' | 'game_start' | 'game_end' | 'game_pause' | 'game_unpause', content: any }}
    */
  const data = JSON.parse(event.data);

  if (data.type === 'session_info') {
    init(data.content.data, data.content.state);
  } else if (data.type === 'score') {
    score(data.content);
  } else if (data.type === 'game_start') {
    startedTime = data.content.time_started;
  } else if (data.type === 'game_end') {
    gameEnded = true;
    startedTime = null;
  } else if (data.type === 'game_pause') {
    gamePaused = true;
  } else if (data.type === 'game_unpause') {
    gamePaused = false;
    timePaused += data.content.paused_time;
  }
});

eventSource.addEventListener('error', _ => {
  window.location.href = '/';
});

/**
  * @param {{ duration: number, score_points: { name: string, category: string, points: number }[] }} data
  * @param {{ blue_scored: { [key: number]: number }, red_scored: { [key: number]: number }, time_started: number?, ended: boolean }} state 
  */
function init(data, state) {
  startedTime = state.time_started;
  scorePoints = data.score_points;
  gameEnded = state.ended;

  document.getElementById('loading').style.display = 'none';
  document.getElementById('main').style.display = 'block';

  points.blue = getScored(state.blue_scored);
  points.red = getScored(state.red_scored);
  updatePoints();

  generateScoreCategories({ red: state.red_scored, blue: state.blue_scored });
  startUpdateTimeInterval(data.duration);
}

function startUpdateTimeInterval(duration) {
  const timeLeftText = document.getElementById('timeLeftText');
  const id = setInterval(() => {
    if (gamePaused) return;

    let text;
    if (startedTime) {
      const timeLeft = duration * 1000 - (Date.now() - (startedTime + timePaused));
      if (timeLeft <= 0) text = '0:00';
      text = formatTime(timeLeft);
    } else if (gameEnded) {
      text = '0:00';
      if (points.red > points.blue) {
        document.getElementById('redAlliance').classList.add('winner');
      } else if (points.blue > points.red) {
        document.getElementById('blueAlliance').classList.add('winner');
      }
      clearInterval(id);
    } else {
      text = formatTime(duration * 1000);
    }
    if (timeLeftText.innerText !== text) timeLeftText.innerText = text;
  }, 1);
}

/**
  * @param {{ red: { [key: number]: number }, blue: { [key: number]: number } }} scored 
  */
function generateScoreCategories(scored) {
  const scoreCategories = document.getElementById('scoreCategories');
  const categories = new Set();

  for (let i = 0; i < scorePoints.length; i++) {
    const { category, points } = scorePoints[i];
    if (categories.has(category)) continue;
    categories.add(category);
    let blueTimesScored = scored.blue[i] ?? 0;
    let redTimesScored = scored.red[i] ?? 0;
    scoreCategories.appendChild(generateCategory(category, { blue: blueTimesScored * points, red: redTimesScored * points }));
  }
}

/**
  * @param {string} category 
  * @param {{ blue: number, red: number }} pointsScored 
  */
function generateCategory(category, pointsScored) {
  const parent = document.createElement('div');
  parent.classList.add('category');

  const name = document.createElement('p');
  name.innerText = category;
  name.classList.add('name');

  const [leftPoints, leftPointsText] = nestedPInDiv(pointsScored.blue);
  leftPointsText.id = `${category}:bluePoints`;
  leftPoints.classList.add('points', 'left');

  const [rightPoints, rightPointsText] = nestedPInDiv(pointsScored.red);
  rightPointsText.id = `${category}:redPoints`;
  rightPoints.classList.add('points', 'right');

  parent.appendChild(leftPoints);
  parent.appendChild(name);
  parent.appendChild(rightPoints);

  return parent;
}

/**
  * @param {{ team: 'blue' | 'red', score_id: number }} content 
  */
function score(content) {
  const scored = scorePoints[content.score_id];
  const pointsScored = scored.points;
  if (content.team === 'blue') {
    points.blue += pointsScored;
  } else if (content.team === 'red') {
    points.red += pointsScored;
  }
  updatePoints();

  const categoryPoints = document.getElementById(`${scored.category}:${content.team}Points`);
  categoryPoints.innerText = +categoryPoints.innerText + pointsScored;
}

function updatePoints() {
  document.getElementById('bluePoints').innerText = points.blue;
  document.getElementById('redPoints').innerText = points.red;
}

/**
  * @param {{ [key: number]: number }} scored 
  */
function getScored(scored) {
  let totalScored = 0;
  for (const [scoreId, timesScored] of Object.entries(scored)) {
    totalScored += scorePoints[parseInt(scoreId)].points * timesScored;
  }
  return totalScored;
}

function nestedPInDiv(text) {
  const div = document.createElement('div');
  const p = document.createElement('p');
  p.innerText = text;
  div.appendChild(p);
  return [div, p];
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

