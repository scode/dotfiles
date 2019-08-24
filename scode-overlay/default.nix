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

  userPackagesForAllPlatforms = super.userPackages or {} // {
    # Must not forget nix itself or nix-env will disappear when nix-rebuild-scode is called.
    nix = super.nix;

    bazel = super.bazel;
    fzf = super.fzf;
    dep = super.dep;
    go = super.go;
    graphviz = super.graphviz;
    emacs = super.emacs;
    i3 = super.i3;
    httpie = super.httpie;
    jq = super.jq;
    lastpass-cli = super.lastpass-cli;
    lzop = super.lzop;
    maven = super.maven;
    mosh = super.mosh;
    mypy = super.mypy;
    openjdk = super.openjdk;
    par2cmdline = super.par2cmdline;
    pdftk = super.pdftk;
    pipenv = super.pipenv;
    pwgen = super.pwgen;
    python3 = super.python3;
    rclone = super.rclone;
    ripgrep = super.ripgrep;
    shellcheck = super.shellcheck;
    tree = super.tree;
    wrk = super.wrk;

    nix-rebuild-scode = super.writeScriptBin "nix-rebuild-scode"
      ''
        #!${super.stdenv.shell}
        exec nix-env -f '<nixpkgs>' -r -iA userPackages "''${@}"
      '';

    saltybox = super.buildGoModule rec {
      name = "saltybox-${version}";
      version = "1.2.2";

      src = super.fetchFromGitHub {
        owner = "scode";
        repo = "saltybox";
        rev = "v${version}";
        sha256 = "1p9a5bcg6c72kpsby3314l4qrvcdv2g611pp5c2psxwjrh8962r1";
      };

      modSha256 = "14c1ak43abkih54lhi8479l6b86qdhrbb7k8hf5rpgqh0ns4fhbs";

      subPackages = [ "." ];
     };
  };

  userPackagesForLinux = {
    dmenu = super.dmenu;
    dunst = super.dunst;
    evince = super.evince;
    firefox = super.firefox;
    flameshot = super.flameshot;
    htop = super.htop;
    i7z = super.i7z;
    minikube = super.minikube;
    mpv = super.mpv;
    kubernetes = super.kubernetes;
    pavucontrol = super.pavucontrol;
    powertop = super.powertop;
    signal-desktop = super.signal-desktop;
    sysstat = super.sysstat;

    backlight_py = self.scriptFromTree {
      name = "backlight.py";
      treePath = "${./backlight.py}";
    };

    xkbcomp_sh = self.scriptFromTree {
      name = "scode-xkbcomp.sh";
      treePath = "${./scode-xkbcomp.sh}";
    };

    op_export_py = self.scriptFromTree {
      name = "op-export.py";
      treePath = "${./op-export.py}";
    };

    google-chrome-slack = super.writeScriptBin "google-chrome-slack"
      ''
        #!${super.stdenv.shell}
        exec google-chrome --user-data-dir=/home/scode/.config/google-chrome-slack
      '';

    open = super.writeScriptBin "open"
      ''
        #!${super.stdenv.shell}
        exec xdg-open "''${@}"
      '';
  };

  userPackages = super.userPackages or {} //
    (if super.stdenv.isLinux then self.userPackagesForLinux else {}) //
    self.userPackagesForAllPlatforms;
}
