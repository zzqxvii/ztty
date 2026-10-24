# Ztty: load the usual startup files, report folder and title, mark prompts and commands.
if [ -n "${ZTTY_BASH_LOGIN-}" ]; then
  builtin unset ZTTY_BASH_LOGIN
  [ -r /etc/profile ] && builtin source /etc/profile
  for _ztty_file in ~/.bash_profile ~/.bash_login ~/.profile; do
    if [ -r "$_ztty_file" ]; then
      builtin source "$_ztty_file"
      break
    fi
  done
  builtin unset _ztty_file
else
  [ -r ~/.bashrc ] && builtin source ~/.bashrc
fi
_ztty_ran=
_ztty_n=0
_ztty_ps0=
# MSYS's bash, Git for Windows' too, names a folder as Windows does with pwd -W.
_ztty_msys=
builtin pwd -W >/dev/null 2>&1 && _ztty_msys=1
if (( BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1] >= 404 )); then
  # PS0 runs in the shell, so its subscript can note that a command ran.
  _ztty_ps0='${_ztty_note[_ztty_ran=1]-}\e]133;C\a'
fi
_ztty_prompt() {
  builtin local ret=$? LC_ALL=C char url= win= i title dir="$PWD"
  if [ -n "$_ztty_ran" ]; then
    builtin printf '\e]133;D;%s\a' "$ret"
    _ztty_ran=
  fi
  [ -n "$_ztty_ps0" ] || _ztty_d=([1]=$'\e]133;D;'"$ret"$'\a')
  if [ -n "${ZTTY_WSL-}" ]; then
    # Under WSL the folder goes as Windows names it: a drive's, or the distribution's share.
    dir=$(wslpath -w "$PWD" 2>/dev/null) || dir=
    dir=${dir//\\//}
    case $dir in //*) ;; ?:*) dir=/$dir ;; *) dir= ;; esac
  elif [ -n "$_ztty_msys" ]; then
    # Git for Windows and MSYS2 name it so too: C:/x, or //host/share; x keeps a name's last newline.
    dir=$(builtin pwd -W; builtin printf x)
    dir=${dir%$'\nx'}
    # Windows holds a name's controls and "*:<>?| at U+F000 up, as MSYS writes them.
    win=${dir:0:2}
    for (( i = 2; i < ${#dir}; i++ )); do
      char=${dir:i:1}
      if [[ $char == [$'\x01'-$'\x1f'] || '"*:<>?|' == *"$char"* ]]; then
        builtin printf -v char '%d' "'$char"
        builtin printf -v char '\\xef\\x%02x\\x%02x' $(( 0x80 + (char >> 6) )) $(( 0x80 + (char & 63) ))
        builtin printf -v char '%b' "$char"
      fi
      win+=$char
    done
    dir=$win
    case $dir in //*) ;; ?:*) dir=/$dir ;; *) dir= ;; esac
  fi
  for (( i = 0; i < ${#dir}; i++ )); do
    char=${dir:i:1}
    case $char in
      [[:cntrl:]]|%) builtin printf -v char '%%%02X' "'$char" ;;
    esac
    url+=$char
  done
  title=${PWD##*/}
  [ -n "$dir" ] && builtin printf '\e]7;file://%s\a' "$url"
  builtin printf '\e]2;%s\a' "${title//[[:cntrl:]]/}"
  builtin return $ret
}
# Last, after anything that sets PS1 or PS0.
_ztty_mark() {
  if [[ $PS1 != *'\e]133;A'* ]]; then
    if [ -z "$_ztty_ps0" ] && builtin shopt -q promptvars; then
      # bash 3.2 has no PS0; \#, the command number, moves once a command ran.
      PS1='\[${_ztty_d[_ztty_n && \# != _ztty_n]-}${_ztty_note[_ztty_n = \#]-}\e]133;A;aid=ztty\a\]'"$PS1"'\[\e]133;B\a\]'
    else
      PS1='\[\e]133;A;aid=ztty\a\]'"$PS1"'\[\e]133;B\a\]'
    fi
  fi
  if [ -n "$_ztty_ps0" ] && builtin shopt -q promptvars && [[ ${PS0-} != *"$_ztty_ps0"* ]]; then
    PS0=$_ztty_ps0${PS0-}
  fi
}
PROMPT_COMMAND="_ztty_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND};_ztty_mark"
# PS0 and the prompt mark a command only through expansions, which promptvars turns on.
builtin shopt -q promptvars || builtin printf 'Ztty: bash marks no command while promptvars is off (shopt -s promptvars)\n' >&2
