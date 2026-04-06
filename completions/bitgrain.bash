# bash completion for bitgrain

_bitgrain()
{
    local cur prev subcmd
    COMPREPLY=()

    # Work with or without bash-completion's _init_completion helper.
    if declare -F _init_completion >/dev/null 2>&1; then
        _init_completion -n : || return
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
        prev="${COMP_WORDS[COMP_CWORD-1]}"
    fi

    # Find first subcommand token.
    subcmd=""
    local w
    for w in "${COMP_WORDS[@]:1}"; do
        case "$w" in
            encode|decode|roundtrip) subcmd="$w"; break ;;
        esac
    done

    local global_flags="-h -v --help --version"
    local legacy_flags="-i -o -d -cd -q -Q -t -m -y --quality --output-quality --threads --deterministic --metrics --overwrite"
    local encode_flags="-o --output -q --quality -t --threads --deterministic -y --overwrite -h --help -v --version"
    local decode_flags="-o --output -Q --output-quality -t --threads --deterministic -y --overwrite -h --help -v --version"
    local roundtrip_flags="-o --output -q --quality -Q --output-quality -t --threads --deterministic -m --metrics -y --overwrite -h --help -v --version"
    local quality_values="50 60 70 75 80 85 90 95 100"
    local thread_values="1 2 4 8 16"
    local subcommands="encode decode roundtrip"

    # Handle --opt=value forms.
    case "$cur" in
        --quality=*)
            COMPREPLY=( $(compgen -W "$quality_values" -- "${cur#--quality=}") )
            COMPREPLY=( "${COMPREPLY[@]/#/--quality=}" )
            return 0
            ;;
        --output-quality=*)
            COMPREPLY=( $(compgen -W "$quality_values" -- "${cur#--output-quality=}") )
            COMPREPLY=( "${COMPREPLY[@]/#/--output-quality=}" )
            return 0
            ;;
        --threads=*)
            COMPREPLY=( $(compgen -W "$thread_values" -- "${cur#--threads=}") )
            COMPREPLY=( "${COMPREPLY[@]/#/--threads=}" )
            return 0
            ;;
    esac

    case "$prev" in
        -o|--output)
            COMPREPLY=( $(compgen -f -- "$cur") )
            COMPREPLY+=( $(compgen -W "-" -- "$cur") )
            return 0
            ;;
        -q|--quality|-Q|--output-quality)
            COMPREPLY=( $(compgen -W "$quality_values" -- "$cur") )
            return 0
            ;;
        -t|--threads)
            COMPREPLY=( $(compgen -W "$thread_values" -- "$cur") )
            return 0
            ;;
        -i)
            COMPREPLY=( $(compgen -f -- "$cur") )
            COMPREPLY+=( $(compgen -d -- "$cur") )
            COMPREPLY+=( $(compgen -W "-" -- "$cur") )
            return 0
            ;;
    esac

    # No subcommand yet: suggest subcommands, global and legacy flags.
    if [[ -z "$subcmd" ]]; then
        if [[ "$cur" == -* ]]; then
            COMPREPLY=( $(compgen -W "$global_flags $legacy_flags $subcommands" -- "$cur") )
        else
            COMPREPLY=( $(compgen -W "$subcommands" -- "$cur") )
            COMPREPLY+=( $(compgen -f -- "$cur") )
            COMPREPLY+=( $(compgen -d -- "$cur") )
        fi
        return 0
    fi

    # Subcommand-specific flags.
    if [[ "$cur" == -* ]]; then
        case "$subcmd" in
            encode)    COMPREPLY=( $(compgen -W "$encode_flags" -- "$cur") ) ;;
            decode)    COMPREPLY=( $(compgen -W "$decode_flags" -- "$cur") ) ;;
            roundtrip) COMPREPLY=( $(compgen -W "$roundtrip_flags" -- "$cur") ) ;;
        esac
        return 0
    fi

    # Positional paths (+ stdin marker).
    COMPREPLY=( $(compgen -f -- "$cur") )
    COMPREPLY+=( $(compgen -d -- "$cur") )
    COMPREPLY+=( $(compgen -W "-" -- "$cur") )
    return 0
}

# Support common invocation styles:
#   bitgrain
#   ./bitgrain
#   /usr/bin/bitgrain
#   /usr/local/bin/bitgrain
complete -o nospace -F _bitgrain bitgrain ./bitgrain /usr/bin/bitgrain /usr/local/bin/bitgrain
