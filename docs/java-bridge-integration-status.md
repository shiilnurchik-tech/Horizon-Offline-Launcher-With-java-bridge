# Java Bridge integration: investigation, not yet implemented

This is a work plan, **not a working combined launcher or a release**. The checkout is
Horizon Offline Launcher at `dfc2b49848d0d53678f45ef11b089f7ed5b636bc`; the Java Bridge
examined is `iaxivers/horizon-java-bridge` branch `java-bridge` at
`99e8ba80497e0515909af4601a4b078020034d2c` (2026-11-10). Pin the commit when importing it; do not silently track the
branch tip.

## Why this is more than bundling two executables

* Java Bridge changes the Horizon game, its shared crates, and the launcher. Its launcher
  launches a separate `java/` client built with `horizon-client/java-mods`, using the
  installed version's prepared files. Copying its launcher modules alone cannot work.
* The offline fork's history was imported as a snapshot, not as a descendant of the bridge
  repository. The offline tree is based on Horizon nightly `fdc87ae` (2026-11-08); the
  bridge's own upstream plan names `41d5649ed5` as its earlier Horizon base. A trial
  three-way tree merge using `41d5649ed5` as an approximate base reports 137 conflicting
  paths (many are translation JSONs). This is an estimate, not a substitute for resolving
  and testing the changes. Conflicts include the launcher home UI, backend, gameplay,
  renderer, Cargo manifests and lockfile. A clean textual merge would still need API and
  behaviour tests.
* In the bridge, `launcher/src/java_variant.rs` requires an owner record, and
  `crates/horizon-jvm/src/gate.rs` independently validates it with Minecraft services.
  The current offline fork deliberately permits play without an account. Thus the two
  behaviours cannot be combined merely by enabling a cargo feature: the desired ownership
  policy must be designed and implemented consistently on **both** sides. The upstream
  `java-mods-dev` bypass is for developer harnesses, not a production integration.
* Minecraft 26.3 files and mods are not included. Bridge mods are beta, require the user's
  own game files and Java 25 JDK, and Fabric components may need a network connection on
  first setup. Offline play must not be described as a guarantee that initial installation
  works without downloads.

## Progress

The bridge's Fabric jar file-management module and its tests have been ported. The CLI
`java-mods list/add/remove` manages files under an instance's `mods-java/` and explicitly
warns that this build cannot run them. The JVM crate's sources have also been imported;
the game does not yet call them. Content-porter, mod block states and collisions, large
in-memory protocol frames, worldgen noise seeds, OBJ texture slots, mod biome and
particle support, data-pack inputs, separate native/Java chunk tickets, and some world
hooks have been ported while retaining newer offline changes. The `Java Bridge integration
checks` workflow tests these components, checks the offline game binary and starts a real
JVM with a stand-in Fabric fixture on Linux. It also checks the file manager on Linux and
Windows. The fixture is not the actual game: the game still does not start the JVM or run
Java mods, and the workflow does not publish a combined build.

## Integration work

1. Port the bridge crate, its game hooks and related workspace dependencies against this
   checkout, resolving conflicts rather than replacing the newer offline tree wholesale.
   Preserve the offline skin/cape, jar selection, protocol and release changes. Review
   `docs/java-bridge-upstream-plan.md` in the bridge source for the hook groups.
2. Port the Java mods instance setting, UI, import/list, launch and prepared-format checks
   into this fork's launcher. Resolve translations as JSON data, preserving both sets of
   keys. Ensure disabling Fabric never starts Java or downloads its dependencies.
3. Agree on the account/ownership behaviour and make the launcher-to-client contract
   explicit, with tests for offline and signed-in starts, refusal/error paths, first-run
   downloads, and opt-in. Bump the bridge's launcher contract if the launch agreement
   changes; do not claim upstream signed binaries implement the changed contract.
4. Build and test matching client and launcher from the same commit. The bridge's Linux
   and Windows release build uses `HORIZON_BRIDGE_PLATFORM=source` and
   `cargo build --release --locked -p horizon-client --features java-mods --bin horizon-client`;
   follow its `java-mods-release.yml` packaging checks so no Mojang or Fabric files are
   shipped. Run launcher and game unit tests and, with separately supplied lawful test
   files, an end-to-end Fabric fixture. Test at least Linux x86-64 and Windows x64;
   the bridge describes macOS as untested.
5. Add an Actions workflow that uploads a clearly labelled matching launcher and Java
   client as run artifacts even without a signing key. For updates/releases, configure a
   repository-controlled signing key and matching public key, choose the release channel
   and repository deliberately, and verify signatures. The current offline launcher
   points to `gerdaroot/Horizon-Offline-Launcher`, while the bridge's fallback points to
   `iaxivers/horizon-java-bridge`; leaving those defaults can replace a combined build or
   download an incompatible client. Never publish an unsigned artifact as a signed release.

## Current verification limits

An experimental paired build now combines this fork's launcher with the pinned upstream
Java Bridge client plus `integration/upstream-offline-java-mods.patch`. It explicitly opts in
with `play --fabric`; normal play does not enable the bridge. The fork's offline mode does
not verify ownership and is distinct from the upstream owner's gate. Both programs are built
from source; Minecraft, Fabric and third-party mods are not bundled. See
`integration/README-ru.txt` for usage and security warnings. The GUI does not yet offer a
Fabric toggle, and the `java-mods` JSON field `bridge_integrated` remains false because this
is an experimental external client, not full integration into the offline game.

The patch's opt-in and upstream owner gate tests passed on Linux and Windows; the patched
client compiled on both in Actions run 43735245210. A launcher Linux test in that run failed
on translation coverage, fixed subsequently. The full Linux launcher test passed in run
43736556798 after generating the stale CLI reference in the runner; the reference has since
been updated and CI now checks it without accepting differences. Run 43737613557 passed all
four Linux/Windows build jobs and both beta bundle jobs; run 43737613525 passed the bridge,
client, launcher file-management and JVM fixture checks. No real game
launch or real Fabric mod has been verified: legal user-supplied Minecraft files, a matching
prepared version, Java 25 JDK and runtime dependencies are still required. Do not describe
these artifacts as a verified working game or signed release.
