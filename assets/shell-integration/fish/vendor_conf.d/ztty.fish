# Ztty: restore XDG_DATA_DIRS, report the folder, mark what this fish leaves unmarked; fish_title keeps the title.
if test -n "$ZTTY_FISH_XDG_DATA_DIRS"
    set -gx XDG_DATA_DIRS $ZTTY_FISH_XDG_DATA_DIRS
else
    set -e XDG_DATA_DIRS
end
set -e ZTTY_FISH_XDG_DATA_DIRS
if status is-interactive
    function _ztty_prompt --on-event fish_prompt
        set -l dir $PWD
        if set -q ZTTY_WSL
            # Under WSL the folder goes as Windows names it: a drive's, or the distribution's share.
            set dir (wslpath -w $PWD 2>/dev/null | string replace -a '\\' / | string collect)
            if string match -qr '^[A-Za-z]:' -- $dir
                set dir /$dir
            else if not string match -qr '^//' -- $dir
                set dir
            end
        end
        test -n "$dir"; and printf '\e]7;file://%s\a' (string escape --style=url -- $dir)
    end
    # fish 4.3 marks its own prompts and commands; 4.0 all but the prompt's end; older ones none.
    set -l v (string split ' ' -- (string replace -r '^(\d+)\.(\d+).*' '$1 $2' -- $version))
    if test $v[1] -lt 4
        set -g _ztty_marks A B
        function _ztty_preexec --on-event fish_preexec
            printf '\e]133;C\a'
        end
        function _ztty_postexec --on-event fish_postexec
            printf '\e]133;D;%s\a' $status
        end
    else if test $v[1] -eq 4 -a $v[2] -lt 3
        set -g _ztty_marks B
    end
    if set -q _ztty_marks
        # The user's prompt loads late and may change; the marks go around the one shown.
        function _ztty_wrap --on-event fish_prompt
            if functions -q _ztty_user_prompt; and string match -q '*_ztty_user_prompt*' -- (functions fish_prompt)
                return
            end
            functions -q _ztty_user_prompt; and functions -e _ztty_user_prompt
            functions -c fish_prompt _ztty_user_prompt
            function fish_prompt
                # First, so the user's prompt sees the command's status.
                set -l text (_ztty_user_prompt | string collect -N)
                contains A $_ztty_marks; and printf '\e]133;A;aid=ztty\a'
                printf '%s\e]133;B\a' "$text"
            end
        end
    end
end
