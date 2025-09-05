using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Sockets;
using System.Text;

namespace Carbon.Plugins {
    [Info("rustctl_sock", "<jalho>", "0.1.0")]
    [Description("Emit server events over a Unix domain socket.")]
    public class rustctl_sock : CarbonPlugin {
        private string plugin_name = "rustctl_sock";
        private Socket socket = null;
        private UnixDomainSocketEndPoint endpoint = null;
        private bool socket_connected = false;

        public rustctl_sock() {
            // TODO: Template the socket path: Should be defined in the Rust
            //       source only! Also template the plugin's name as it must
            //       correspond to the written file's name!
            this.endpoint = new UnixDomainSocketEndPoint("/tmp/rustctl.sock");
            this.init_socket();
        }

        private void init_socket() {
            try {
                this.socket = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
                this.socket.Connect(this.endpoint);
                this.socket_connected = true;
                this.log("Successfully connected to Unix domain socket");
            } catch (Exception ex) {
                this.log($"Failed to connect to Unix domain socket: {ex.Message}");
                this.socket_connected = false;
            }
        }

        private bool ensure_connection() {
            if (this.socket_connected && this.socket?.Connected == true) {
                return true;
            }

            try {
                this.socket?.Close();
                this.socket?.Dispose();
                this.init_socket();
                return this.socket_connected;
            } catch (Exception ex) {
                this.log($"Failed to reconnect socket: {ex.Message}");
                this.socket_connected = false;
                return false;
            }
        }

        private void write_hook_data(object event_data) {
            if (!this.ensure_connection()) return;

            try {
                string json = JsonConvert.SerializeObject(event_data) + "\n";
                byte[] bytes = Encoding.UTF8.GetBytes(json);
                this.socket.Send(bytes);
            } catch (SocketException ex) {
                this.log($"Socket error writing event: {ex.Message}");
                this.socket_connected = false;
            } catch (Exception ex) {
                this.log($"Error writing to socket: {ex.Message}");
            }
        }

        object OnDispenserGather(ResourceDispenser resource_dispenser, BasePlayer player, Item item) {
            this.write_hook_data(
                new Dictionary<string, object> {
                    ["hook"] = "OnDispenserGather",
                    ["item.amount"] = item.amount,
                    ["item.info.displayName.english"] = item.info.displayName.english,
                    ["player.Connection.userid"] = player.Connection.userid
                }
            );
            return null;
        }

        void OnDispenserBonus(ResourceDispenser resource_dispencer, BasePlayer player, Item item) {
            this.write_hook_data(
                new Dictionary<string, object> {
                    ["hook"] = "OnDispenserBonus",
                    ["item.amount"] = item.amount,
                    ["item.info.displayName.english"] = item.info.displayName.english,
                    ["player.Connection.userid"] = player.Connection.userid
                }
            );
        }

        // object OnPlayerDeath(BasePlayer killed_player, HitInfo killer_info) {
        //     bool is_killer_player = killer_info?.InitiatorPlayer?.userID is ulong
        //         && !killer_info.InitiatorPlayer.IsNpc;
        //     bool is_suicide = is_killer_player
        //         && killer_info.InitiatorPlayer.userID == killed_player.userID;

        //     if (is_killer_player && !is_suicide) {
        //         this.write_hook_data("OnPlayerDeath", new {
        //             kind = "pvp",
        //             killer_id = killer_info.InitiatorPlayer.userID.ToString(),
        //             killed_id = killed_player.userID.ToString()
        //         });
        //     } else {
        //         string majority_damage_type;
        //         if (killer_info == null) {
        //             majority_damage_type = "unknown";
        //         } else {
        //             majority_damage_type = killer_info.damageTypes.GetMajorityDamageType().ToString();
        //         }
        //         this.write_hook_data("OnPlayerDeath", new {
        //             kind = "pve",
        //             damage_type = majority_damage_type,
        //             killed_id = killed_player.userID.ToString()
        //         });
        //     }
        //     return null;
        // }

        // object OnGrowableGathered(GrowableEntity growable, Item gathered, BasePlayer player) {
        //     this.write_hook_data("OnGrowableGathered", new {
        //         player_id = player.userID.ToString(),
        //         item_shortname = gathered.info.shortname,
        //         quantity = gathered.amount
        //     });
        //     return null;
        // }

        // object OnCollectiblePickup(CollectibleEntity collectible, BasePlayer player, bool eat) {
        //     this.write_hook_data("OnCollectiblePickup", new {
        //         player_id = player.userID.ToString(),
        //         item_name = collectible.name,
        //         quantity = 1
        //     });
        //     return null;
        // }

        // void OnCargoShipSpawnCrate(CargoShip self) {
        //     this.write_hook_data("OnCargoShipSpawnCrate", new {
        //         // TODO: Serialize whatever the `CargoShip self` contains?
        //     });
        // }

        public void Unload() {
            try {
                this.socket?.Shutdown(SocketShutdown.Both);
            } catch { }

            try {
                this.socket?.Close();
            } catch { }

            this.socket?.Dispose();
        }

        private void log(string message) {
            string timestamp_iso = DateTime.UtcNow.ToString("o");
            System.Console.WriteLine($"[{timestamp_iso}] {this.plugin_name}: {message}");
        }
    }
}

