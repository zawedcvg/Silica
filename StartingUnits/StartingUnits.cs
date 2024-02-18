//a good portion of the code has been adapted from https://github.com/data-bomb/Silica/blob/main/Si_SpawnConfigs/Si_SpawnConfigs.cs

using MelonLoader;
using HarmonyLib;
using SilicaAdminMod;
using UnityEngine;
using MelonLoader.Utils;
using System;
using System.Collections.Generic;
using System.Linq;
using System.IO;
using Newtonsoft.Json;
using Il2Cpp;
using Si_StartingUnits;
using System.Runtime.CompilerServices;


[assembly: MelonInfo(typeof(StartingUnits), "Starting units", "0.4.13", "zawedcvg")]
[assembly: MelonGame("Bohemia Interactive", "Silica")]
[assembly: MelonOptionalDependencies("Admin Mod")]

namespace Si_StartingUnits
{
    public class StartingUnits : MelonMod
    {
        static bool AdminModAvailable = false;
        static GameObject? lastSpawnedObject;
        private String? defaultStartingUnits;
        public static StartingUnits? Instance;
        public bool loaded = false;
        //private MelonPreferences_Entry<String>? defaultStartingUnitsConfig;


        public class StartingSpawnSetup
        {
            public List<TeamSpawn>? Teams
            {
                get;
                set;
            }

            public List<RelativeUnitSpawn>? RelativeUnits
            {
                get;
                set;
            }
        }

        public class TeamSpawn
        {
            public int TeamIndex
            {
                get;
                set;
            }

            public int Resources
            {
                get;
                set;
            }
        }

        public class ObjectSpawn
        {
            private string _classname = null!;

            public String Classname
            {
                get => _classname;
                set => _classname = value ?? throw new ArgumentNullException("Classname name is required.");
            }

            private float[] _position = null!;

            public float[] Position
            {
                get => _position;
                set => _position = value ?? throw new ArgumentNullException("Position is required.");
            }

            private float[] _rotation = null!;

            public float[] Rotation
            {
                get => _rotation;
                set => _rotation = value ?? throw new ArgumentNullException("Rotation is required.");
            }

            public uint? NetID
            {
                get;
                set;
            }
        }


        public class RelativeUnitSpawn : ObjectSpawn
        {
            public int TeamIndex
            {
                get;
                set;
            }

            public int? Resources
            {
                get;
                set;
            }
        }


        static string GetSpawnConfigsDirectory()
        {
            return Path.Combine(MelonEnvironment.UserDataDirectory, @"StartingUnits\");
        }


        public void getDefaultConfig()
        {
            String startingUnitsConfig = GetSpawnConfigsDirectory();
            //ourFirstCategory = MelonPreferences.CreateCategory("OurFirstCategory");


            if (!System.IO.Directory.Exists(startingUnitsConfig))
            {
                MelonLogger.Msg("Creating StartingUnitsConfig directory at: " + startingUnitsConfig);
                System.IO.Directory.CreateDirectory(startingUnitsConfig);
            }

            String configFile = Path.Combine(startingUnitsConfig, @"defaultstarting.cfg");

            // recheck this part, feels iffy
            //defaultStartingUnits = MelonPreferences.CreateCategory("defaultStartingUnits");
            String default_thing = "None";

            if (!File.Exists(configFile))
            {
                File.WriteAllText(configFile, default_thing);
            }
            defaultStartingUnits = File.ReadAllText(configFile);

            MelonLogger.Msg("Loaded till here");

            MelonLogger.Msg($"config file is {configFile}");


        }

        public void Command_SetDefault(Player callerPlayer, String args)
        {
            // validate argument count
            int argumentCount = args.Split(' ').Length - 1;
            if (argumentCount > 1)
            {
                HelperMethods.ReplyToCommand(args.Split(' ')[0] + ": Too many arguments");
                return;
            }

            else if (argumentCount < 1)
            {
                HelperMethods.ReplyToCommand(args.Split(' ')[0] + ": Too few arguments");
                return;
            }

            String defaultConfigName = args.Split(' ')[1];

            String startingUnitsConfig = GetSpawnConfigsDirectory();
            String configFile = Path.Combine(startingUnitsConfig, defaultConfigName + @".json");
            String defaultConfigFile = Path.Combine(startingUnitsConfig, @"defaultstarting.cfg");
            MelonLogger.Msg($"default file path is {configFile}");

            if (!File.Exists(configFile))
            {
                MelonLogger.Msg($"No file exists at {configFile}");
                return;
            }

            //if (defaultStartingUnits)

            //MelonLogger

            File.WriteAllText(defaultConfigFile, defaultConfigName);

        }

        public override void OnLateInitializeMelon()
        {
            AdminModAvailable = RegisteredMelons.Any(m => m.Info.Name == "Admin Mod");
            MelonLogger.Msg("Getting the default value");
            getDefaultConfig();
            Instance = this;

            if (AdminModAvailable)
            {
                HelperMethods.CommandCallback startingSpawnCallback = Command_Spawn;
                HelperMethods.RegisterAdminCommand("!startingunitspawn", startingSpawnCallback, Power.Cheat);

                HelperMethods.CommandCallback undoSpawnCallback = Command_UndoSpawn;
                HelperMethods.RegisterAdminCommand("!undospawn", undoSpawnCallback, Power.Cheat);

                HelperMethods.CommandCallback saveCallback = Command_SaveSetup;
                HelperMethods.RegisterAdminCommand("!saveconfig", saveCallback, Power.Cheat);

                HelperMethods.CommandCallback removeUnitsCallback = Command_RemoveUnits;
                HelperMethods.RegisterAdminCommand("!removeallunits", removeUnitsCallback, Power.Cheat);

                HelperMethods.CommandCallback setDefaultConfigCallBack = Command_SetDefault;
                HelperMethods.RegisterAdminCommand("!setdefault", setDefaultConfigCallBack, Power.Cheat);
            }
            else
            {
                MelonLogger.Warning("Dependency missing: Admin Mod");
            }
        }

        public void Command_RemoveUnits(Player callerPlayer, String args)
        {
            // validate argument count
            int argumentCount = args.Split(' ').Length - 1;
            if (argumentCount > 0)
            {
                HelperMethods.ReplyToCommand(args.Split(' ')[0] + ": Too many arguments");
                return;
            }

            StartingSpawnSetup startingSpawn = GenerateSpawnSetup(true);

            RemoveUnits(startingSpawn);

            HelperMethods.AlertAdminAction(callerPlayer, "removed all the units");
        }

        public static void RemoveUnits(StartingSpawnSetup removeSetup)
        {
            if (removeSetup.RelativeUnits != null)
            {
                foreach (RelativeUnitSpawn spawnEntry in removeSetup.RelativeUnits)
                {
                    if (spawnEntry.NetID == null)
                    {
                        continue;
                    }

                    MelonLogger.Msg("Removing unit " + spawnEntry.Classname);

                    Unit thisUnit = Unit.GetUnitByNetID((uint)spawnEntry.NetID);
                    thisUnit.DamageManager.SetHealth01(0.0f);
                }
            }
        }

        public static void Command_UndoSpawn(Player callerPlayer, String args)
        {
            // validate argument count
            int argumentCount = args.Split(' ').Length - 1;
            if (argumentCount > 0)
            {
                HelperMethods.ReplyToCommand(args.Split(' ')[0] + ": Too many arguments");
                return;
            }

            if (lastSpawnedObject == null)
            {
                HelperMethods.ReplyToCommand(args.Split(' ')[0] + ": Nothing to undo");
                return;
            }

            BaseGameObject baseObject = lastSpawnedObject.GetBaseGameObject();
            String name = baseObject.ToString();
            baseObject.DamageManager.SetHealth01(0.0f);
            lastSpawnedObject = null;

            HelperMethods.AlertAdminAction(callerPlayer, "destroyed last spawned item (" + name + ")");
        }
        public static void Command_Spawn(Player callerPlayer, String args)
        {
            // validate argument count
            int argumentCount = args.Split(' ').Length - 1;
            if (argumentCount > 1)
            {
                HelperMethods.ReplyToCommand(args.Split(' ')[0] + ": Too many arguments");
                return;
            }
            else if (argumentCount < 1)
            {
                HelperMethods.ReplyToCommand(args.Split(' ')[0] + ": Too few arguments");
                return;
            }

            Vector3 playerPosition = callerPlayer.GetComponent<Transform>().position;
            Quaternion playerRotation = callerPlayer.GetComponent<Transform>().rotation;
            String spawnName = args.Split(' ')[1];

            int teamIndex = callerPlayer.Team.Index;
            GameObject? spawnedObject = HelperMethods.SpawnAtLocation(spawnName, playerPosition, playerRotation, teamIndex);
            if (spawnedObject == null)
            {
                HelperMethods.ReplyToCommand(args.Split(' ')[0] + ": Failed to spawn");
                return;
            }

            HelperMethods.AlertAdminAction(callerPlayer, "spawned " + spawnName);
        }
        public void Command_SaveSetup(Player callerPlayer, String args)
        {
            String commandName = args.Split(' ')[0];

            // validate argument count
            int argumentCount = args.Split(' ').Length - 1;
            if (argumentCount > 1)
            {
                HelperMethods.ReplyToCommand(commandName + ": Too many arguments");
                return;
            }
            else if (argumentCount < 1)
            {
                HelperMethods.ReplyToCommand(commandName + ": Too few arguments");
                return;
            }

            String configFile = args.Split(' ')[1];

            try
            {
                // check if UserData\SpawnConfigs\ directory exists
                String spawnConfigDir = GetSpawnConfigsDirectory();
                if (!System.IO.Directory.Exists(spawnConfigDir))
                {
                    MelonLogger.Msg("Creating SpawnConfigs directory at: " + spawnConfigDir);
                    System.IO.Directory.CreateDirectory(spawnConfigDir);
                }

                // check if file extension is valid
                if (configFile.Contains('.') && !configFile.EndsWith("json"))
                {
                    HelperMethods.ReplyToCommand(commandName + ": Invalid save name (not .json)");
                    return;
                }

                // add .json if it's not already there
                if (!configFile.Contains('.'))
                {
                    configFile += ".json";
                }

                // final check on filename
                if (configFile.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
                {
                    HelperMethods.ReplyToCommand(commandName + ": Cannot use input as filename");
                    return;
                }

                // don't overwrite an existing file
                String configFileFullPath = Path.Combine(spawnConfigDir, configFile);
                if (File.Exists(configFileFullPath))
                {
                    HelperMethods.ReplyToCommand(commandName + ": configuration already exists. Updating it");
                }
                else
                {
                    HelperMethods.ReplyToCommand(commandName + ": configuration does not exists. Creating new configuration it");

                }

                // is there anything to save right now?
                if (!GameMode.CurrentGameMode.GameOngoing)
                {
                    HelperMethods.ReplyToCommand(commandName + ": Nothing to save with current game state");
                    return;
                }


                StartingSpawnSetup spawnSetup = GenerateSpawnSetup();

                // save to file
                String JsonRaw = JsonConvert.SerializeObject(spawnSetup, Newtonsoft.Json.Formatting.Indented);

                File.WriteAllText(configFileFullPath, JsonRaw);

                HelperMethods.ReplyToCommand(commandName + ": Saved config to file");
            }
            catch (Exception error)
            {
                HelperMethods.PrintError(error, "Command_SaveSetup failed");
            }
        }


        public static void RemoveConstructionSites()
        {
            MelonLogger.Msg("Removing all construciton sites");
            ConstructionSite.ClearAllConstructionSites();
        }

        public void loadDefaultSetUp()
        {

            String startingUnitsConfig = GetSpawnConfigsDirectory();
            String defaultConfigFile = Path.Combine(startingUnitsConfig, @"defaultstarting.cfg");
            String configFile = File.ReadAllText(defaultConfigFile);
            MelonLogger.Msg($"Default file i am reading from is {configFile}");

            try
            {
                String spawnConfigDir = GetSpawnConfigsDirectory();

                // check if file extension is valid
                if (configFile.Equals("None"))
                {
                    MelonLogger.Msg("There is no file set as default");
                    return;
                }

                


                if (configFile.Contains('.') && !configFile.EndsWith("json"))
                {
                    HelperMethods.ReplyToCommand(": Invalid save name (not .json)");
                    return;
                }

                // add .json if it's not already there
                if (!configFile.Contains('.'))
                {
                    configFile += ".json";
                }

                // final check on filename
                if (configFile.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0)
                {
                    HelperMethods.ReplyToCommand(": Cannot use input as filename");
                    return;
                }

                // do we have anything to load here?
                String configFileFullPath = System.IO.Path.Combine(spawnConfigDir, configFile);
                if (!File.Exists(configFileFullPath))
                {
                    HelperMethods.ReplyToCommand(": configuration not found");
                    return;
                }

                // check global config options

                String JsonRaw = File.ReadAllText(configFileFullPath);
                MelonLogger.Msg("Loaded till here");
                StartingSpawnSetup? spawnSetup = JsonConvert.DeserializeObject<StartingSpawnSetup>(JsonRaw);

                if (spawnSetup == null)
                {
                    HelperMethods.ReplyToCommand("Default file is empty");
                    return;
                }

                String[] requiredTeams = new String[spawnSetup.Teams.Count];
                int index = 0;

                foreach (TeamSpawn teamSpawn in spawnSetup.Teams)
                {
                    requiredTeams[index] = Team.Teams[teamSpawn.TeamIndex].TeamName;
                    MelonLogger.Msg(requiredTeams[index]);
                    index += 1;
                }

                StartingSpawnSetup originalSpawnSetup = GenerateSpawnSetup(true, requiredTeams);

                RemoveUnits(originalSpawnSetup);

                // load new units
                if (!LoadUnits(spawnSetup))
                {
                    HelperMethods.ReplyToCommand(": invalid unit in config file");
                    return;
                }

                // set anything team-specific
                //LoadTeams(spawnSetup);

                HelperMethods.ReplyToCommand(": Loaded config from file");
            }
            catch (Exception error)
            {
                HelperMethods.PrintError(error, "Command_LoadSetup failed");
            }
        }

        public static void LoadTeams(StartingSpawnSetup addSetup)
        {
            // team-specific info
            if (addSetup.Teams != null)
            {
                foreach (TeamSpawn spawnEntry in addSetup.Teams)
                {
                    Team.Teams[spawnEntry.TeamIndex].StartingResources = spawnEntry.Resources;
                }
            }
        }

        public static bool LoadUnits(StartingSpawnSetup addSetup)
        {
            // load all units
            if (addSetup.RelativeUnits != null)
            {
                foreach (RelativeUnitSpawn spawnEntry in addSetup.RelativeUnits)
                {
                    Team team = Team.Teams[spawnEntry.TeamIndex];

                    Structure structure_team = null;
                    foreach (Structure structure in team.Structures)
                    {
                        structure_team = structure;
                    }
                    if (structure_team == null)
                    {
                        MelonLogger.Msg("Team has no structures");

                    }

                    MelonLogger.Msg("Adding unit " + spawnEntry.Classname);

                    Vector3 position = new Vector3
                    {
                        x = spawnEntry.Position[0] + structure_team.WorldPhysicalCenter.x,
                        y = spawnEntry.Position[1] + structure_team.WorldPhysicalCenter.y,
                        z = spawnEntry.Position[2] + structure_team.WorldPhysicalCenter.z
                    };
                    Quaternion rotation = new Quaternion
                    {
                        x = spawnEntry.Rotation[0],
                        y = spawnEntry.Rotation[1],
                        z = spawnEntry.Rotation[2],
                        w = spawnEntry.Rotation[3]
                    };
                    GameObject? spawnedObject = HelperMethods.SpawnAtLocation(spawnEntry.Classname, position, rotation, spawnEntry.TeamIndex);
                    if (spawnedObject == null)
                    {
                        return false;
                    }

                    BaseGameObject baseObject = spawnedObject.GetBaseGameObject();

                    if (baseObject.IsResourceHolder && spawnEntry.Resources != null)
                    {
                        // assign biotics (Resources[1]) to alien team and balterium (Resources[0]) to human teams
                        Resource resource = spawnEntry.TeamIndex == 0 ? Resource.Resources[1] : Resource.Resources[0];
                        baseObject.StoreResource(resource, (int)spawnEntry.Resources);
                    }
                }
            }

            return true;
        }

        public List<String> getTeamsInvolved()
        {
            MP_Strategy strategyInstance = GameObject.FindObjectOfType<MP_Strategy>();
            MP_Strategy.ETeamsVersus teamVersusMode = strategyInstance.TeamsVersus;
            List<String> required_teams = new List<String>();

            if (teamVersusMode == MP_Strategy.ETeamsVersus.HUMANS_VS_HUMANS)
            {
                MelonLogger.Msg("In here mate");
                required_teams.Add(Team.Teams[1].TeamName);
                required_teams.Add(Team.Teams[2].TeamName);
            }
            else if (teamVersusMode == MP_Strategy.ETeamsVersus.HUMANS_VS_HUMANS_VS_ALIENS)
            {
                required_teams.Add(Team.Teams[1].TeamName);
                required_teams.Add(Team.Teams[2].TeamName);

            }
            else
            {
                required_teams.Add(Team.Teams[1].TeamName);
                required_teams.Add(Team.Teams[2].TeamName);
            }

            return required_teams;


        }


        public StartingSpawnSetup GenerateSpawnSetup(bool includeNetIDs = false, params String[] teams)
        {
            // set global config options
            StartingSpawnSetup spawnSetup = new StartingSpawnSetup();
            spawnSetup.Teams = new List<TeamSpawn>();
            MP_Strategy strategyInstance = GameObject.FindObjectOfType<MP_Strategy>();
            spawnSetup.RelativeUnits = new List<RelativeUnitSpawn>();
            String teamVersusMode = strategyInstance.TeamsVersus.ToString();
            MelonLogger.Msg($"Team versus mode is {teamVersusMode}");
            List<String> teamsInvolved = getTeamsInvolved();
            // create a list of all structures and units
            MelonLogger.Msg("Going inside the loop");

            foreach (Team team in Team.Teams)
            {

                if (teams.Length != 0 && !teams.Contains(team.TeamName))
                {
                    MelonLogger.Msg("We are here, wtf");
                    MelonLogger.Msg($"We are here, wtf {teams != null}");
                    continue;
                }

                if (!teamsInvolved.Contains(team.TeamName))
                {
                    MelonLogger.Msg("team not a part of team involve");
                    MelonLogger.Msg($"{team.TeamName}");
                    continue;
                }

                if (team == null)
                {
                    continue;
                }

                TeamSpawn thisTeamSpawn = new TeamSpawn
                {
                    Resources = team.StartingResources,
                    TeamIndex = team.Index
                };
                spawnSetup.Teams.Add(thisTeamSpawn);

                foreach (Unit unit in team.Units)
                {
                    Player player = unit.m_ControlledBy;
                    if (player != null) { continue; }
                    RelativeUnitSpawn thisSpawnEntry = new RelativeUnitSpawn();

                    Structure structure_team = null;
                    foreach (Structure structure in team.Structures)
                    {
                        structure_team = structure;
                    }
                    
                    if(structure_team == null)
                    {
                        MelonLogger.Msg("Team has no structures");

                    }

                    float[] position = new float[]
                    {
                         unit.transform.position.x - structure_team.WorldPhysicalCenter.x,
                         unit.transform.position.y - structure_team.WorldPhysicalCenter.y,
                            unit.transform.position.z - structure_team.WorldPhysicalCenter.z,
                    };
                    thisSpawnEntry.Position = position;

                    Quaternion facingRotation = unit.GetFacingRotation();
                    float[] rotation = new float[]
                    {
                        facingRotation.x,
                        facingRotation.y,
                        facingRotation.z,
                        facingRotation.w
                    };
                    thisSpawnEntry.Rotation = rotation;

                    thisSpawnEntry.TeamIndex = unit.Team.Index;
                    thisSpawnEntry.Classname = unit.ToString().Split('(')[0];
                    

                    if (includeNetIDs)
                    {
                        thisSpawnEntry.NetID = unit.NetworkComponent.NetID;
                    }

                    // only record health if damaged

                    if (unit.IsResourceHolder)
                    {
                        thisSpawnEntry.Resources = unit.GetResourceCapacity();
                    }

                    spawnSetup.RelativeUnits.Add(thisSpawnEntry);
                }
            }
            return spawnSetup;
        }

        [HarmonyPatch(typeof(StrategyTeamSetup), nameof(StrategyTeamSetup.SpawnAIUnits))]
        private static class ApplyPatchOnLateUpdatel3
        {
            public static void Postfix(StrategyTeamSetup __instance)
            {
                MelonLogger.Msg("Loading from default setup");
                if (!Instance.loaded)
                {
                Instance.loadDefaultSetUp();
                }

                Instance.loaded = true; 
            }

        }

        [HarmonyPatch(typeof(MusicJukeboxHandler), nameof(MusicJukeboxHandler.OnGameStarted))]
        private static class ApplyPatchOnLateUpdatel4
        {
            public static void Postfix()
            {
                Instance.loaded = false;
            }

        }
        [HarmonyPatch("Il2CppInterop.HarmonySupport.Il2CppDetourMethodPatcher", "ReportException")]
        internal static class Patch_Il2CppDetourMethodPatcher
        {
            public static bool Prefix(System.Exception ex)
            {
                MelonLogger.Error("During invoking native->managed trampoline", ex);
                return false;
            }
        }

    }
}