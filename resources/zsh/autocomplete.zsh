_iris_completion() {
    local line state
    _arguments -C \
        '1: :->command' \
        '*:: :->args'

    case "$state" in
        args)
            case "$line[1]" in
                switch|preview)
                    _arguments '1:theme:($(iris complete-list 2>/dev/null))'
                    ;;
                apply)
                    _arguments \
                        '(-t --theme)'{-t,--theme}'[Override active theme]:theme:($(iris complete-list 2>/dev/null))' \
                        '(-b --fallback)'{-b,--fallback}'[Use fallback theme]'
            ;;
        esac
    ;;
esac
}
compdef _iris_completion iris
