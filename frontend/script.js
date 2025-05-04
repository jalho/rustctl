let ws = null;
let availableCommands = [];

function connectWebSocket() {
  ws = new WebSocket("/sock");

  ws.onmessage = function(event) {
    let data = JSON.parse(event.data);

    if (data.game && data.game.data.commands_available) {
      updateAvailableCommands(data.game.data.commands_available);
    }

    if (data.game && data.game.data.players) {
      updatePlayersTable(data.game.data.players);
    }
  };
}

function sendCommand(cmd) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ _type: cmd }));
  }
}

function updateAvailableCommands(commands) {
  availableCommands = commands.map(command => command._type);
  const commandButtons = document.querySelectorAll('.command-button');

  commandButtons.forEach(button => {
    const command = button.getAttribute('data-command');
    if (availableCommands.includes(command)) {
      button.disabled = false;
    } else {
      button.disabled = true;
    }
  });
}

function updatePlayersTable(players) {
  let tbody = document.querySelector("table tbody");
  tbody.innerHTML = "";

  for (let playerId in players) {
    let player = players[playerId];
    let row = document.createElement("tr");

    let statusCell = document.createElement("td");
    let statusDot = document.createElement("span");
    statusDot.classList.add("status-dot", "online");
    statusCell.appendChild(statusDot);
    statusCell.append(player.display_name);
    row.appendChild(statusCell);

    let countryCell = document.createElement("td");
    countryCell.textContent = getFlagEmoji(player.country);
    row.appendChild(countryCell);

    let idCell = document.createElement("td");
    idCell.classList.add("steam-id");
    idCell.textContent = player.id;
    row.appendChild(idCell);

    tbody.appendChild(row);
  }
}

function getFlagEmoji(code) {
  return code
    .toUpperCase()
    .slice(0, 2)
    .split("")
    .map(c => String.fromCodePoint(0x1F1E6 + c.charCodeAt(0) - 65))
    .join("");
}

window.addEventListener("load", connectWebSocket);
