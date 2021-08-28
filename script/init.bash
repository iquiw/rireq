_rireq_preexec() {
	rireq record "$1"
}

_rireq_history() {
	tput smcup
	HISTORY="$(rireq history | fzf +s -q "$READLINE_LINE")"
	tput rmcup
	READLINE_LINE=${HISTORY}
	READLINE_POINT=${#READLINE_LINE}
}

_rireq_setup() {
	local found=0
	for f in ${preexec_functions[*]}; do
		if [ "$f" = _rireq_preexec ]; then
			found=1
			break
		fi
	done
	case $found in
	0)	preexec_functions+=(_rireq_preexec) ;;
	esac

	bind -x '"\C-r": _rireq_history'
}

_rireq_setup
