# Install overlay by symlinking the containing directory as  ~/.config/nixpkgs/overlays/scode
# and for initial install run nix-env -f '<nixpkgs>' -iA userPackages
#
# Note that nix-rebuild-scode will remove any packages not part of userPackages.

self: super:

{

  # Install a single-file executable script contained in this overlay
  # tree itself. It's basically writeScriptBin except allows us to put
  # the file next to this .nix file.
  #
  # Note: This does zero magic to pull in appropriate dependencies, rewrite
  # shebangs, etc. It's merely a helper to install a file, period.
  #
  # (Should find a cleaner way of doing this, probably by creating a
  # fetchFromTree (ala fetchFromGitHub) if/when I figure out how to do
  # so correctly.)
  scriptFromTree =
    {
      name,
      treePath
    }:
    super.stdenv.mkDerivation rec {
      inherit name;
      phases = [ "buildPhase" ];

      buildPhase = ''
        mkdir -p $out/bin
        echo cp ${treePath} $out/bin/${name}
        cp ${treePath} $out/bin/${name}
      '';
    };

  userPackages = super.userPackages or {} // {
    # Must not forget nix itself or nix-env will disappear when nix-rebuild-scode is called.
    nix = super.nix;

    ripgrep = super.ripgrep;

    # Demonstration use of scriptFromTree:
    #
    #scode-helloworld = self.scriptFromTree {
    #  name = "scode-helloworld";
    #  treePath = "${./scode-helloworld.py}";
    #};

    nix-rebuild-scode = super.writeScriptBin "nix-rebuild-scode"
      ''
        #!${super.stdenv.shell}
        exec nix-env -f '<nixpkgs>' -r -iA userPackages
      '';

    google-chrome-slack = super.writeScriptBin "google-chrome-slack"
      ''
        #!${super.stdenv.shell}
        exec google-chrome --user-data-dir=/home/scode/.config/google-chrome-slack
      '';

    saltybox = super.buildGoPackage rec {
      name = "saltybox-${version}";
      version = "1.0.0";

      goPackagePath = "github.com/scode/saltybox";

      src = super.fetchFromGitHub {
        owner = "scode";
        repo = "saltybox";
        rev = "v${version}";
        sha256 = "0a2p2hicw71vzycxg4xafl7xj4y3ky7h9v67g1b1ai1pwabx08sc";
      };

      buildFlags = "--tags release";
     };
  };
}