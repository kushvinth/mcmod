I want to build a declarative Minecraft mod manager for macOS.

My Minecraft directory is:

~/Library/Application Support/Minecraft/

The directory contains:

* A global `mods/` folder
* An `instances/` directory
* Multiple Minecraft instances inside `instances/`
* Each instance may have its own `mods/` folder

Example structure:

Minecraft/
├── mods/
├── instances/
│   ├── Build/
│   │   └── mods/
│   ├── pvp/
│   │   └── mods/
│   └── wow/
│       └── mods/

I want a single declarative configuration file that defines which mods belong in which instance.

For example, something conceptually like:

```yaml
minecraft_version: "1.21.11"
loader: fabric

common:
  - fabric-api
  - sodium
  - lithium
  - iris
  - modmenu

instances:
  Build:
    mods:
      - appleskin
      - xaeros-minimap
      - xaeros-world-map

  pvp:
    mods:
      - combat-hitbox
      - freecam
      - meteor-client

  wow:
    mods:
      - gamma-utils
      - shulkerboxtooltip
```

The exact configuration format can be YAML, TOML, JSON, or another format if there is a strong technical reason.

The tool should behave like a declarative package manager or infrastructure-as-code tool. The config file is the desired state, and the program reconciles the actual Minecraft installation with that desired state.

Required functionality:

1. Discover all Minecraft instances dynamically by scanning:

   `~/Library/Application Support/Minecraft/instances/`

   Do not hardcode instance names.

2. Detect each instance's `mods/` directory.

3. Support a global/default mod set that can be shared across instances.

4. Support instance-specific mod sets.

5. Install missing mods automatically.

6. Update managed mods when newer compatible versions are available.

7. Remove obsolete managed mods that are no longer declared in the configuration.

8. Never accidentally delete unmanaged `.jar` files. The tool must distinguish between managed and unmanaged mods, possibly through a lockfile or state file.

9. Resolve mods from Modrinth where possible. If a mod cannot be resolved automatically, support explicit download URLs or local JAR paths.

10. Respect Minecraft version and mod-loader compatibility when resolving versions.

11. Handle dependencies automatically where possible.

12. Use atomic or safe file operations so that a failed download or interrupted update does not leave the mods directory in a broken state.

13. Generate and maintain a lockfile containing exact resolved versions, project IDs, file hashes, download URLs, and target instances.

14. Provide commands similar to:

```bash
mcmods init
mcmods plan
mcmods apply
mcmods update
mcmods status
```

Expected semantics:

* `init`: inspect the current Minecraft installation and generate an initial configuration and lockfile from existing mods.
* `plan`: show what would be installed, updated, moved, or removed without modifying anything.
* `apply`: reconcile all instances with the declared configuration.
* `update`: resolve newer compatible versions and update the lockfile.
* `status`: show drift between the declared state and actual instance mod folders.

Important design requirements:

* The program must correctly handle any number of instances.
* Instance discovery must be dynamic.
* Paths containing spaces must work correctly.
* The tool should be idempotent: running `apply` twice should produce no changes the second time.
* Shared mods should not require duplicated declarations in the configuration.
* The program should produce clear errors when a mod is incompatible with the configured Minecraft version or Fabric version.
* Before destructive operations, provide a dry-run/plan phase.
* Managed files should be verified using cryptographic hashes.
* Downloads should first go to a temporary location and only be moved into place after verification.

My current installation already contains many manually downloaded JAR files. The initial migration workflow is important: the tool should inspect existing JARs, identify them through metadata inside the JAR (`fabric.mod.json`) and/or Modrinth API matching, and generate the initial declarative config without forcing me to manually re-enter every mod.

Please design this as a proper maintainable CLI application rather than a one-off shell script.

Before implementing, provide:

1. Recommended architecture.
2. Recommended language and justification.
3. Configuration schema.
4. Lockfile schema.
5. State reconciliation algorithm.
6. Existing-mod import strategy.
7. Safety model for deletion and rollback.
8. Project directory structure.
9. Then implement the CLI incrementally, starting with instance discovery and current-state scanning.