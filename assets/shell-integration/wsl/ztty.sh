# Ztty: start this distribution's login shell with Ztty's integration, which WSLENV hands over as ZTTY_WSL.
case ${SHELL##*/} in
  zsh)
    [ -n "${ZDOTDIR+set}" ] && export ZTTY_ZSH_ZDOTDIR="$ZDOTDIR"
    export ZDOTDIR="$ZTTY_WSL/zsh"
    exec "$SHELL" -l
    ;;
  bash)
    export ZTTY_BASH_LOGIN=1
    exec "$SHELL" --rcfile "$ZTTY_WSL/bash/ztty.bash"
    ;;
  fish)
    export ZTTY_FISH_XDG_DATA_DIRS="${XDG_DATA_DIRS-}"
    export XDG_DATA_DIRS="$ZTTY_WSL:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
    exec "$SHELL" -l
    ;;
esac
printf 'Ztty: no integration for %s; its folder goes unreported\n' "${SHELL:?Ztty: WSL names no login shell}" >&2
exec "$SHELL" -l
