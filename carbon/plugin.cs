using System;
using System.Text;
using System.IO;
using Newtonsoft.Json;
using System.Net.Sockets;
using System.Reflection;

class JSONSerializable {
    public virtual string to_json() {
        return JsonConvert.SerializeObject(this);
    }
}

enum Category {
    /**
     * Case "player killed another player".
     */
    PvP,
    /**
     * Case e.g. "player got killed by NPC".
     */
    PvE,
    /**
     * Case e.g. "player collected wood".
     */
    Farm,
    /**
     * Case e.g. "crate spawned on cargo ship".
     */
    World,
}

class PlayerEventPvpKill : JSONSerializable {
    [JsonProperty("type")]
    public string Type { get; } = "PlayerEventPvpKill";

    public Category category { get; set; }

    public ulong timestamp { get; set; }

    /** SteamID of the killer player. */
    public string id_subject { get; set; }

    /** SteamID of the killed player. */
    public string id_object { get; set; }
}

class PlayerEventPveDeath : JSONSerializable {
    [JsonProperty("type")]
    public string Type { get; } = "PlayerEventPveDeath";

    public Category category { get; set; }

    public ulong timestamp { get; set; }

    /** Some identifier of the killer. */
    public string id_subject { get; set; }

    /** SteamID of the killed player. */
    public string id_object { get; set; }
}

class PlayerEventFarming : JSONSerializable {
    [JsonProperty("type")]
    public string Type { get; } = "PlayerEventFarming";

    public Category category { get; set; }

    public ulong timestamp { get; set; }

    /** SteamID of the farming player. */
    public string id_subject { get; set; }

    /** Some identifier of what was farmed. */
    public string id_object { get; set; }

    /** How much was farmed. */
    public int quantity { get; set; }
}

class WorldEvent : JSONSerializable {
    [JsonProperty("type")]
    public string Type { get; } = "WorldEvent";

    public Category category { get; set; }

    public ulong timestamp { get; set; }

    /** Some identifier of the event. */
    public string id_subject { get; set; }
}

namespace Carbon.Plugins {
    [Info ( "rustctl_sock", "<jalho>", "0.1.0" )]
    [Description ( "Emit server events over a Unix domain socket." )]
    public class rustctl_sock : CarbonPlugin {
        private string plugin_name = "rustctl_sock";
        private Socket socket = null;
        private UnixDomainSocketEndPoint endpoint = null;
        private bool socket_connected = false;

        private MemoryStream memory_stream = new MemoryStream();
        private StreamWriter stream_writer;
        private JsonSerializer json_serializer;

        public rustctl_sock() {
            this.endpoint = new UnixDomainSocketEndPoint("/tmp/rustctl.sock");

            this.json_serializer = JsonSerializer.Create();
            this.stream_writer = new StreamWriter(this.memory_stream, Encoding.UTF8);

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

        /**
         * Carbon hook called when a player gathers from a "dispenser", i.e.
         * e.g. a tree or a stone node.
         */
        object OnDispenserGather(ResourceDispenser resource_dispenser, BasePlayer player, Item item) {
            long timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var farming_event = new PlayerEventFarming {
                category = Category.Farm,
                timestamp = (ulong) timestamp,
                id_subject = (player.userID).ToString(),
                id_object = item.info.shortname,
                quantity = item.amount,
            };
            this.write_sock(farming_event);
            return (object) null;
        }

        /**
         * Carbon hook called e.g. when a player hits a tree for the last time
         * so that it falls down (as opposed to the initial hit, or its
         * subsequent hits that don't yet fall the tree).
         */
        void OnDispenserBonus(ResourceDispenser resource_dispencer, BasePlayer player, Item item) {
            long timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var farming_event = new PlayerEventFarming {
                category = Category.Farm,
                timestamp = (ulong) timestamp,
                id_subject = (player.userID).ToString(),
                id_object = item.info.shortname,
                quantity = item.amount,
            };
            this.write_sock(farming_event);
        }

        /**
         * Carbon hook called when a player gets killed.
         */
        object OnPlayerDeath(BasePlayer killed_player, HitInfo killer_info) {
            long timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

            bool is_killer_player = killer_info?.InitiatorPlayer?.userID is ulong
                && !killer_info.InitiatorPlayer.IsNpc;
            bool is_suicide = is_killer_player
                && killer_info.InitiatorPlayer.userID == killed_player.userID;

            // case PvP
            if (is_killer_player && !is_suicide) {
                var death_event = new PlayerEventPvpKill {
                    category = Category.PvP,
                    timestamp = (ulong) timestamp,
                    id_subject = killer_info.InitiatorPlayer.userID.ToString(),
                    id_object = killed_player.userID.ToString(),
                };
                this.write_sock(death_event);
            }
            // case PvE
            else {
                string majority_damage_type;
                if (killer_info == null) {
                    majority_damage_type = "unknown PvE damage"; // ??
                } else {
                    majority_damage_type = killer_info.damageTypes.GetMajorityDamageType().ToString();
                }
                var death_event = new PlayerEventPveDeath {
                    category = Category.PvE,
                    timestamp = (ulong) timestamp,
                    id_subject = majority_damage_type,
                    id_object = killed_player.userID.ToString(),
                };
                this.write_sock(death_event);
            }
            return (object) null;
        }

        object OnGrowableGathered(GrowableEntity growable, Item gathered, BasePlayer player) {
            long timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var farming_event = new PlayerEventFarming {
                category = Category.Farm,
                timestamp = (ulong) timestamp,
                id_subject = (player.userID).ToString(),
                id_object = gathered.info.shortname,
                quantity = gathered.amount,
            };
            this.write_sock(farming_event);
            return (object) null;
        }

        /**
         * Carbon hook called e.g. when a player picks up a mushroom or a stump
         * (wood).
         */
        object OnCollectiblePickup(CollectibleEntity collectible, BasePlayer player, bool eat) {
            long timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var farming_event = new PlayerEventFarming {
                category = Category.Farm,
                timestamp = (ulong) timestamp,
                id_subject = (player.userID).ToString(),
                id_object = collectible.name,
                quantity = 1,
            };
            this.write_sock(farming_event);
            return (object) null;
        }

        object OnCargoShipSpawnCrate(CargoShip self) {
            // this.inspect_object(self);
            long timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var world_event = new WorldEvent {
                category = Category.World,
                timestamp = (ulong) timestamp,
                id_subject = "OnCargoShipSpawnCrate",
            };
            this.write_sock(world_event);
            return (object) null;
        }

        /**
         * Debug helper method.
         */
        private void inspect_object(object inspectable) {
            Type inspectable_type = inspectable.GetType();
            PropertyInfo[] properties = inspectable_type.GetProperties();
            StringBuilder property_names = new StringBuilder();
            foreach (PropertyInfo property in properties)
            {
                property_names.Append(property.Name + "\n\t");
            }
            Console.WriteLine($"FullName: '{inspectable_type.FullName}', Property Names:\n\t{property_names}");
        }

        /**
         * Called by Carbon to perform any plugin cleanup at unload.
         */
        public void Unload() {
            try {
                this.socket?.Shutdown(SocketShutdown.Both);
            } catch {
                // socket might already be closed
            }

            try {
                this.socket?.Close();
            } catch {
                // socket might already be closed
            }

            this.socket?.Dispose();

            // clean up reusable resources
            this.stream_writer?.Dispose();
            this.memory_stream?.Dispose();
        }

        private void log(string message) {
            string timestamp_iso = DateTime.UtcNow.ToString("o");
            System.Console.WriteLine($"[{timestamp_iso}] {this.plugin_name}: {message}");
        }

        private void write_sock(JSONSerializable message) {
            if (!this.ensure_connection()) {
                return; // skip if socket not available
            }

            try {
                this.memory_stream.SetLength(0);
                this.memory_stream.Position = 0;

                using (var json_writer = new JsonTextWriter(this.stream_writer)) {
                    json_writer.CloseOutput = false;
                    this.json_serializer.Serialize(json_writer, message);
                    json_writer.Flush();
                }

                this.stream_writer.Flush();

                byte[] data = this.memory_stream.ToArray();

                byte[] data_with_newline = new byte[data.Length + 1];
                Array.Copy(data, data_with_newline, data.Length);
                data_with_newline[data.Length] = (byte)'\n';

                this.socket.Send(data_with_newline);

            } catch (SocketException ex) {
                this.log($"Socket error writing event: {ex.Message}");
                this.socket_connected = false;
            } catch (Exception ex) {
                this.log($"Error writing to Unix domain socket: {ex.Message}");
            }
        }
    }
}
