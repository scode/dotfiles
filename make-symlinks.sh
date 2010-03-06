#!/usr/bin/env zsh

# Trivial helper to quickly populate $(pwd) with symlinks to the dot
# files in the repository.

rcfiles_path=$(dirname $0)

function die {
    echo "$*" 1>&2
    exit
}

case $(uname)
in
    Linux)
        SED="sed -r"
        ;;
    FreeBSD)
        SED="sed -E"
        ;;
    Darwin)
        SED="sed -E"
        ;;
    *)
        die "unknown uname"
esac

if ! [ "$1" = "--force" ]
then
    die 'must say --force to confirm that you understand that we are going to put symlinks in the current directory'
fi

for dotfile in $rcfiles_path/dot-*
do
    name=$(basename $dotfile)

    stripped=$(echo $name | ${=SED} -e 's,dot-(.+)$,\1,g')
    symname=".${stripped}"

    if [ -L $symname ]
    then
        if ! [ "$(readlink $symname)" = "$dotfile" ]
        then
            if [ -d "$symname" ]
            then
                echo "! $symname - is symlink pointing to directory; not touching"
            else
                echo "M $symname -> $dotfile (used to be $(readlink $symname))"
                ln -sf $dotfile $symname
            fi
        else
            echo "  $symname"
        fi
    elif ! [ -e "$symname" ]
    then
        echo "C $symname -> $dotfile"
        ln -sf $dotfile $symname
    else
        echo "! $symname - exists but is not symlink, not touching"
    fi
done
