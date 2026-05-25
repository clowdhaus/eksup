# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_eksup_global_optspecs
	string join \n v/verbose q/quiet h/help V/version
end

function __fish_eksup_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_eksup_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_eksup_using_subcommand
	set -l cmd (__fish_eksup_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c eksup -n "__fish_eksup_needs_command" -s v -l verbose -d 'Increase logging verbosity'
complete -c eksup -n "__fish_eksup_needs_command" -s q -l quiet -d 'Decrease logging verbosity'
complete -c eksup -n "__fish_eksup_needs_command" -s h -l help -d 'Print help'
complete -c eksup -n "__fish_eksup_needs_command" -s V -l version -d 'Print version'
complete -c eksup -n "__fish_eksup_needs_command" -f -a "analyze" -d 'Analyze an Amazon EKS cluster for potential upgrade issues'
complete -c eksup -n "__fish_eksup_needs_command" -f -a "create" -d 'Create artifacts using the analysis data'
complete -c eksup -n "__fish_eksup_needs_command" -f -a "completion" -d 'Generate shell completion script for the given shell'
complete -c eksup -n "__fish_eksup_needs_command" -f -a "man" -d 'Generate the man page for eksup'
complete -c eksup -n "__fish_eksup_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s c -l cluster -d 'The name of the cluster to analyze' -r
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s r -l region -d 'The AWS region where the cluster is provisioned' -r
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s p -l profile -d 'The AWS profile to use to access the cluster' -r
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s f -l format -r -f -a "json\t'JSON format used for logging or writing to a *.json file'
text\t'Text format used for writing to stdout'"
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s o -l output -d 'Write to file instead of stdout' -r
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s t -l target-version -d 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1' -r
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -l config -d 'Path to an eksup configuration file (default: .eksup.yaml in cwd)' -r
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -l ignore-recommended -d 'Exclude recommendations from the output'
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -l show-suppressed -d 'Include findings suppressed by .eksup.yaml ignore rules'
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s v -l verbose -d 'Increase logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s q -l quiet -d 'Decrease logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c eksup -n "__fish_eksup_using_subcommand analyze" -s V -l version -d 'Print version'
complete -c eksup -n "__fish_eksup_using_subcommand create; and not __fish_seen_subcommand_from playbook help" -s v -l verbose -d 'Increase logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand create; and not __fish_seen_subcommand_from playbook help" -s q -l quiet -d 'Decrease logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand create; and not __fish_seen_subcommand_from playbook help" -s h -l help -d 'Print help'
complete -c eksup -n "__fish_eksup_using_subcommand create; and not __fish_seen_subcommand_from playbook help" -s V -l version -d 'Print version'
complete -c eksup -n "__fish_eksup_using_subcommand create; and not __fish_seen_subcommand_from playbook help" -f -a "playbook" -d 'Create a playbook for upgrading an Amazon EKS cluster'
complete -c eksup -n "__fish_eksup_using_subcommand create; and not __fish_seen_subcommand_from playbook help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s c -l cluster -d 'The name of the cluster to analyze' -r
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s r -l region -d 'The AWS region where the cluster is provisioned' -r
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s p -l profile -d 'The AWS profile to use to access the cluster' -r
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s f -l filename -d 'Name of the playbook saved locally' -r
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s t -l target-version -d 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1' -r
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -l config -d 'Path to an eksup configuration file (default: .eksup.yaml in cwd)' -r
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -l ignore-recommended -d 'Exclude recommendations from the output'
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -l show-suppressed -d 'Include findings suppressed by .eksup.yaml ignore rules'
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s v -l verbose -d 'Increase logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s q -l quiet -d 'Decrease logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s h -l help -d 'Print help'
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from playbook" -s V -l version -d 'Print version'
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from help" -f -a "playbook" -d 'Create a playbook for upgrading an Amazon EKS cluster'
complete -c eksup -n "__fish_eksup_using_subcommand create; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c eksup -n "__fish_eksup_using_subcommand completion" -s v -l verbose -d 'Increase logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand completion" -s q -l quiet -d 'Decrease logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand completion" -s h -l help -d 'Print help'
complete -c eksup -n "__fish_eksup_using_subcommand completion" -s V -l version -d 'Print version'
complete -c eksup -n "__fish_eksup_using_subcommand man" -s v -l verbose -d 'Increase logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand man" -s q -l quiet -d 'Decrease logging verbosity'
complete -c eksup -n "__fish_eksup_using_subcommand man" -s h -l help -d 'Print help'
complete -c eksup -n "__fish_eksup_using_subcommand man" -s V -l version -d 'Print version'
complete -c eksup -n "__fish_eksup_using_subcommand help; and not __fish_seen_subcommand_from analyze create completion man help" -f -a "analyze" -d 'Analyze an Amazon EKS cluster for potential upgrade issues'
complete -c eksup -n "__fish_eksup_using_subcommand help; and not __fish_seen_subcommand_from analyze create completion man help" -f -a "create" -d 'Create artifacts using the analysis data'
complete -c eksup -n "__fish_eksup_using_subcommand help; and not __fish_seen_subcommand_from analyze create completion man help" -f -a "completion" -d 'Generate shell completion script for the given shell'
complete -c eksup -n "__fish_eksup_using_subcommand help; and not __fish_seen_subcommand_from analyze create completion man help" -f -a "man" -d 'Generate the man page for eksup'
complete -c eksup -n "__fish_eksup_using_subcommand help; and not __fish_seen_subcommand_from analyze create completion man help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c eksup -n "__fish_eksup_using_subcommand help; and __fish_seen_subcommand_from create" -f -a "playbook" -d 'Create a playbook for upgrading an Amazon EKS cluster'
