/*
    $ node --version
    v20.14.0
*/

import * as http from "node:http";

function info(message, ...args) {
    console.log("[%s] - " + message, new Date().toISOString(), ...args);
}

function handle_connection(i) {
    info("Got TCP connection from %s %s", i.remoteAddress, i.remotePort);
    i.destroy();
}

(function main() {
    const server = http.createServer();
    server.addListener("listening", () => info("RCON mock listening on port %s", server.address().port));
    server.addListener("connection", handle_connection)
    server.listen({ host: "127.0.0.1", port: process.argv[2] ?? 28016 });
})();
