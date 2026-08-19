# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_irongall_global_optspecs
    string join \n h/help V/version
end

function __fish_irongall_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_irongall_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_irongall_using_subcommand
    set -l cmd (__fish_irongall_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c irongall -n "__fish_irongall_needs_command" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_needs_command" -s V -l version -d 'Print version'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "tui" -d 'Launch the TUI (default when no subcommand is given)'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "status" -d 'Print global theme/font/size, fc-match, and an apps summary'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "apply" -d 'Apply the current (or given) selection to every discovered program'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "rollback" -d 'Restore files from the last apply session'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "theme" -d 'Browse, preview, and apply color schemes'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "font" -d 'Browse installed / market fonts'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "size" -d 'Global size'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "apps" -d 'Discover installed themable programs'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "app" -d 'Per-program tweaks'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "market" -d 'Marketplace index (no money)'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "preview" -d 'Print a 16-color ANSI preview without opening the TUI'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "completions" -d 'Generate shell completions'
complete -c irongall -n "__fish_irongall_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand tui" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand status" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand apply" -l theme -r
complete -c irongall -n "__fish_irongall_using_subcommand apply" -l font -r
complete -c irongall -n "__fish_irongall_using_subcommand apply" -l size -r
complete -c irongall -n "__fish_irongall_using_subcommand apply" -l dry-run
complete -c irongall -n "__fish_irongall_using_subcommand apply" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand rollback" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand theme; and not __fish_seen_subcommand_from list show apply search install help" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand theme; and not __fish_seen_subcommand_from list show apply search install help" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and not __fish_seen_subcommand_from list show apply search install help" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and not __fish_seen_subcommand_from list show apply search install help" -f -a "apply"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and not __fish_seen_subcommand_from list show apply search install help" -f -a "search"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and not __fish_seen_subcommand_from list show apply search install help" -f -a "install"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and not __fish_seen_subcommand_from list show apply search install help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from apply" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from search" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from install" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from help" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from help" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from help" -f -a "apply"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from help" -f -a "search"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from help" -f -a "install"
complete -c irongall -n "__fish_irongall_using_subcommand theme; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand font; and not __fish_seen_subcommand_from list show apply search install import help" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand font; and not __fish_seen_subcommand_from list show apply search install import help" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand font; and not __fish_seen_subcommand_from list show apply search install import help" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand font; and not __fish_seen_subcommand_from list show apply search install import help" -f -a "apply"
complete -c irongall -n "__fish_irongall_using_subcommand font; and not __fish_seen_subcommand_from list show apply search install import help" -f -a "search"
complete -c irongall -n "__fish_irongall_using_subcommand font; and not __fish_seen_subcommand_from list show apply search install import help" -f -a "install"
complete -c irongall -n "__fish_irongall_using_subcommand font; and not __fish_seen_subcommand_from list show apply search install import help" -f -a "import" -d 'Copy a directory of fonts you already own into the user font dir'
complete -c irongall -n "__fish_irongall_using_subcommand font; and not __fish_seen_subcommand_from list show apply search install import help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from apply" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from search" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from install" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from import" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from help" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from help" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from help" -f -a "apply"
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from help" -f -a "search"
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from help" -f -a "install"
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from help" -f -a "import" -d 'Copy a directory of fonts you already own into the user font dir'
complete -c irongall -n "__fish_irongall_using_subcommand font; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand size; and not __fish_seen_subcommand_from set help" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand size; and not __fish_seen_subcommand_from set help" -f -a "set"
complete -c irongall -n "__fish_irongall_using_subcommand size; and not __fish_seen_subcommand_from set help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand size; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand size; and __fish_seen_subcommand_from help" -f -a "set"
complete -c irongall -n "__fish_irongall_using_subcommand size; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand apps" -l json
complete -c irongall -n "__fish_irongall_using_subcommand apps" -l all -d 'Include programs that are not installed'
complete -c irongall -n "__fish_irongall_using_subcommand apps" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand app; and not __fish_seen_subcommand_from list show set reset skip help" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand app; and not __fish_seen_subcommand_from list show set reset skip help" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand app; and not __fish_seen_subcommand_from list show set reset skip help" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand app; and not __fish_seen_subcommand_from list show set reset skip help" -f -a "set"
complete -c irongall -n "__fish_irongall_using_subcommand app; and not __fish_seen_subcommand_from list show set reset skip help" -f -a "reset"
complete -c irongall -n "__fish_irongall_using_subcommand app; and not __fish_seen_subcommand_from list show set reset skip help" -f -a "skip"
complete -c irongall -n "__fish_irongall_using_subcommand app; and not __fish_seen_subcommand_from list show set reset skip help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from list" -l json
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from list" -l all
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from set" -l theme -r
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from set" -l font -r
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from set" -l size -r
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from set" -l follow
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from set" -l hold
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from set" -l dry-run
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from set" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from reset" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from skip" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from help" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from help" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from help" -f -a "set"
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from help" -f -a "reset"
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from help" -f -a "skip"
complete -c irongall -n "__fish_irongall_using_subcommand app; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand market; and not __fish_seen_subcommand_from update help" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand market; and not __fish_seen_subcommand_from update help" -f -a "update"
complete -c irongall -n "__fish_irongall_using_subcommand market; and not __fish_seen_subcommand_from update help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand market; and __fish_seen_subcommand_from update" -l url -r
complete -c irongall -n "__fish_irongall_using_subcommand market; and __fish_seen_subcommand_from update" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand market; and __fish_seen_subcommand_from help" -f -a "update"
complete -c irongall -n "__fish_irongall_using_subcommand market; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand preview; and not __fish_seen_subcommand_from theme help" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand preview; and not __fish_seen_subcommand_from theme help" -f -a "theme"
complete -c irongall -n "__fish_irongall_using_subcommand preview; and not __fish_seen_subcommand_from theme help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand preview; and __fish_seen_subcommand_from theme" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand preview; and __fish_seen_subcommand_from help" -f -a "theme"
complete -c irongall -n "__fish_irongall_using_subcommand preview; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand completions" -s h -l help -d 'Print help'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "tui" -d 'Launch the TUI (default when no subcommand is given)'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "status" -d 'Print global theme/font/size, fc-match, and an apps summary'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "apply" -d 'Apply the current (or given) selection to every discovered program'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "rollback" -d 'Restore files from the last apply session'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "theme" -d 'Browse, preview, and apply color schemes'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "font" -d 'Browse installed / market fonts'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "size" -d 'Global size'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "apps" -d 'Discover installed themable programs'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "app" -d 'Per-program tweaks'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "market" -d 'Marketplace index (no money)'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "preview" -d 'Print a 16-color ANSI preview without opening the TUI'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "completions" -d 'Generate shell completions'
complete -c irongall -n "__fish_irongall_using_subcommand help; and not __fish_seen_subcommand_from tui status apply rollback theme font size apps app market preview completions help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from theme" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from theme" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from theme" -f -a "apply"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from theme" -f -a "search"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from theme" -f -a "install"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from font" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from font" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from font" -f -a "apply"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from font" -f -a "search"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from font" -f -a "install"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from font" -f -a "import" -d 'Copy a directory of fonts you already own into the user font dir'
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from size" -f -a "set"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from app" -f -a "list"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from app" -f -a "show"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from app" -f -a "set"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from app" -f -a "reset"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from app" -f -a "skip"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from market" -f -a "update"
complete -c irongall -n "__fish_irongall_using_subcommand help; and __fish_seen_subcommand_from preview" -f -a "theme"
