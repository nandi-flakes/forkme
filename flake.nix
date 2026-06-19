{
  description = "forkme - a tool for managing forks using a patch-based approach";

  nixConfig = {
    extra-substituters = [
      "https://codegod100.cachix.org"
    ];
    extra-trusted-public-keys = [
      "codegod100.cachix.org-1:LZFL5VrR644WUjleS3bLbVeOdzlXqzKznQWvD5MVthA="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          forkme = pkgs.rustPlatform.buildRustPackage {
            pname = "forkme";
            version = "0.2.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = with pkgs; [ perl pkg-config ];
            meta = with pkgs.lib; {
              description = "A tool for managing forks using a patch-based approach";
              homepage = "https://tangled.org/me.webbeef.org/forkme";
              license = licenses.agpl3Only;
              mainProgram = "forkme";
              platforms = platforms.unix;
            };
          };
        in
        {
          inherit forkme;
          default = forkme;
        }
        // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          oci = pkgs.dockerTools.buildLayeredImage {
            name = "forkme";
            tag = "latest";
            contents = [ forkme pkgs.cacert pkgs.gitMinimal ];
            config = {
              Entrypoint = [ "${pkgs.lib.getExe forkme}" ];
              Env = [
                "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              ];
              WorkingDir = "/work";
            };
          };
        });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.forkme}/bin/forkme";
        };
      });
    };
}
