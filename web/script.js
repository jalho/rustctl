let ws = null;
let availableCommands = [];

function connectWebSocket() {
  ws = new WebSocket("/sock");

  ws.onmessage = function(event) {
    let data = JSON.parse(event.data);

    // Update available commands if provided in the state update
    if (data.game && data.game.data.commands_available) {
      updateAvailableCommands(data.game.data.commands_available);
    }

    // Update players information if provided in the state update
    if (data.game && data.game.data.players) {
      updatePlayersTable(data.game.data.players);
    }
  };
}

function sendCommand(cmd) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: cmd }));
  }
}

function updateAvailableCommands(commands) {
  availableCommands = commands.map(command => command.type); // Extract only the "type" field
  const commandButtons = document.querySelectorAll('.command-button');

  commandButtons.forEach(button => {
    const command = button.getAttribute('data-command');
    if (availableCommands.includes(command)) {
      button.disabled = false;  // Enable the button if the command is available
    } else {
      button.disabled = true;   // Disable the button if the command is not available
    }
  });
}

function updatePlayersTable(players) {
  let tbody = document.querySelector("table tbody");
  tbody.innerHTML = "";  // Clear existing rows

  for (let playerId in players) {
    let player = players[playerId];
    let row = document.createElement("tr");

    // Player status cell (online/offline dot and player name)
    let statusCell = document.createElement("td");
    let statusDot = document.createElement("span");
    statusDot.classList.add("status-dot", "online");  // Assuming all players are online
    statusCell.appendChild(statusDot);
    statusCell.append(player.display_name);
    row.appendChild(statusCell);

    // Country cell (using the `getFlagEmoji` function)
    let countryCell = document.createElement("td");
    countryCell.textContent = getFlagEmoji(player.country);
    row.appendChild(countryCell);

    // Steam ID cell
    let idCell = document.createElement("td");
    idCell.classList.add("steam-id");
    idCell.textContent = player.id;
    row.appendChild(idCell);

    // Append the row to the table
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
