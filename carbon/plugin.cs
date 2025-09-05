using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Sockets;
using System.Text;

static class JsonHelpers
{
    public static string serialize_as_much_as_possible(object obj)
    {
        var settings = new JsonSerializerSettings
        {
            Error = (sender, args) => { args.ErrorContext.Handled = true; },
            Formatting = Formatting.None
        };
        return JsonConvert.SerializeObject(obj, settings);
    }
}

namespace Carbon.Plugins {
    [Info("rustctl_sock", "<jalho>", "0.1.0")]
    [Description("Emit server events over a Unix domain socket.")]
    public class rustctl_sock : CarbonPlugin {
        private string plugin_name = "rustctl_sock";
        private Socket socket = null;
        private UnixDomainSocketEndPoint endpoint = null;
        private bool socket_connected = false;

        public rustctl_sock() {
            /*
             * TODO: Template the socket path: Should be defined in the Rust
             *       source only! Also template the plugin's name as it must
             *       correspond to the written file's name!
             */
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
                    ["player.Connection.userid"] = player.Connection.userid,
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
                    ["player.Connection.userid"] = player.Connection.userid,
                }
            );
        }

        object OnGrowableGathered(GrowableEntity growable, Item item, BasePlayer player) {
            this.write_hook_data(
                new Dictionary<string, object> {
                    ["hook"] = "OnGrowableGathered",

                    ["item.amount"] = item.amount,
                    ["item.info.displayName.english"] = item.info.displayName.english,
                    ["player.Connection.userid"] = player.Connection.userid,
                }
            );
            return null;
        }

        void OnCargoShipSpawnCrate(CargoShip self) {
            this.write_hook_data(
                new Dictionary<string, object> {
                    ["hook"] = "OnCargoShipSpawnCrate",
                }
            );
        }

        object OnCollectiblePickup(CollectibleEntity item, BasePlayer player, bool eat) {
            this.write_hook_data(
                new Dictionary<string, object> {
                    ["hook"] = "OnCollectiblePickup",
                    ["player.Connection.userid"] = player.Connection.userid,
                    ["item.itemList"] = item.itemList,
                }
            );
            return null;
        }

        /*
         * TODO: Use more hooks... Some interesting ones I've used before:
         *       - OnPlayerDeath
         *
         * Browse more here: https://carbonmod.gg/references/hooks
         */

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

