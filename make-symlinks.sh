#!/usr/bin/env zsh

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

    if ! [ -e $symname ]
    then
        echo "$symname <- $dotfile"
        ln -s $dotfile $symname
    else
        echo "$symname exists"
    fi
done
