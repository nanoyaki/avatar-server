{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-parts.inputs.nixpkgs-lib.follows = "nixpkgs";
    systems.url = "github:nix-systems/default-linux";

    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{ flake-parts, ... }:

    flake-parts.lib.mkFlake { inherit inputs; } (
      { withSystem, ... }:

      {
        systems = import inputs.systems;

        imports = [
          inputs.treefmt-nix.flakeModule
        ];

        flake.overlays = rec {
          default =
            _: prev:

            withSystem prev.stdenv.hostPlatform.system (
              { config, ... }:

              {
                inherit (config.packages) avatar-server;
              }
            );

          avatar-server = default;
        };

        flake.nixosModules = rec {
          default =
            {
              lib,
              pkgs,
              config,
              ...
            }:

            let
              inherit (lib)
                mkEnableOption
                mkPackageOption
                mkOption
                types
                mkIf
                mkDefault
                ;

              cfg = config.services.tangled.avatar-server;
            in

            {
              options.services.tangled.avatar-server = {
                enable = mkEnableOption "avatar-server";

                package = mkPackageOption (withSystem pkgs.stdenv.hostPlatform.system (
                  { config, ... }: config.packages
                )) "avatar-server" { };

                environmentFile = mkOption {
                  description = "A file containing the {env}`AVATAR_SHARED_SECRET` environment variable";
                  example = "/run/secrets/rendered/avatar-server.env";
                  type = types.externalPath;
                };

                port = mkOption {
                  description = "The port the server listens on";
                  example = 8080;
                  default = 3000;
                  type = types.port;
                };
              };

              config = mkIf cfg.enable {
                systemd.services.avatar-server = {
                  description = "Avatar server for Tangled";

                  wantedBy = [ "multi-user.target" ];
                  after = [ "network.target" ];

                  environment = {
                    PORT = toString cfg.port;
                    CACHE_DIR = mkDefault "/var/cache/avatar-server";
                  };

                  serviceConfig = {
                    Type = "simple";
                    ExecStart = lib.getExe cfg.package;
                    Restart = "on-failure";
                    CacheDirectory = mkDefault "avatar-server";
                    EnvironmentFile = cfg.environmentFile;

                    # Hardening
                    DynamicUser = true;
                    CapabilityBoundingSet = "";
                    SystemCallFilter = [ "@system-service" ];

                    NoNewPrivileges = true;
                    ProtectClock = true;
                    RestrictNamespaces = true;
                    RestrictSUIDSGID = true;
                    LockPersonality = true;
                    RestrictRealtime = true;
                    RestrictAddressFamilies = [
                      "AF_INET"
                      "AF_INET6"
                    ];
                    MemoryDenyWriteExecute = true;
                    ProtectHostname = true;

                    ProtectSystem = "strict";
                    PrivateTmp = true;
                    ProtectHome = true;
                    PrivateDevices = true;
                    ProtectControlGroups = true;
                    ProtectKernelTunables = true;
                    ProtectKernelModules = true;
                    ProtectKernelLogs = true;
                    ProtectProc = "invisible";
                  };
                };
              };
            };

          avatar-server = default;
        };

        perSystem =
          {
            lib,
            pkgs,
            common,
            ...
          }:

          {
            _module.args.common = {
              src = lib.cleanSource ./.;

              nativeBuildInputs = with pkgs; [ pkg-config ];
              buildInputs = with pkgs; [ openssl ];
            };

            packages.avatar-server = pkgs.callPackage (
              {
                lib,
                rustPlatform,
              }:

              let
                cargoToml = lib.importTOML ./Cargo.toml;
              in

              rustPlatform.buildRustPackage (
                common
                // {
                  pname = cargoToml.package.name;
                  inherit (cargoToml.package) version;

                  cargoHash = "sha256-2XS97owOxQErQ+a0OSmZBx6XAXxWTzXKrHqgX/KRllw=";

                  meta = {
                    mainProgram = "avatar-server";
                    description = "Alternative avatar server implementation for tangled";
                    maintainers = with lib.maintainers; [ nanoyaki ];
                    license = lib.licenses.agpl3Plus;
                    # Maybe works on others too? Haven't tested it
                    platforms = lib.platforms.linux;
                  };
                }
              )
            ) { };

            devShells.default = pkgs.mkShell {
              packages =
                common.nativeBuildInputs
                ++ common.buildInputs
                ++ (with pkgs; [
                  rustc
                  cargo
                  clippy
                  rustfmt
                  rust-analyzer-unwrapped
                ]);

              RUST_BACKTRACE = 1;
              RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc.outPath;
            };

            treefmt = {
              projectRootFile = "Cargo.toml";
              programs = {
                actionlint.enable = true;
                nixfmt.enable = true;
                rustfmt.enable = true;
              };
            };
          };
      }
    );
}
