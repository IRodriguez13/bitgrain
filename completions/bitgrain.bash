# bash completion for bitgrain

_bitgrain()
{
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    case "$prev" in
        -i|-o)
            COMPREPLY=( $(compgen -f -- "$cur") )
            return 0
            ;;
        -q|-Q)
            COMPREPLY=( $(compgen -W "1 2 3 4 5 10 15 20 25 30 35 40 45 50 55 60 65 70 75 80 85 90 95 100" -- "$cur") )
            return 0
            ;;
    esac

    if [[ "$cur" == -* ]]; then
        COMPREPLY=( $(compgen -W "-i -o -d -cd -q -Q -m -y -v -h" -- "$cur") )
        return 0
    fi

    COMPREPLY=( $(compgen -f -- "$cur") )
    return 0
}

complete -F _bitgrain bitgrain
