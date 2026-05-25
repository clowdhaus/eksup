
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'eksup' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'eksup'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'eksup' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Analyze an Amazon EKS cluster for potential upgrade issues')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create artifacts using the analysis data')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion script for the given shell')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Generate the man page for eksup')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'eksup;analyze' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'The name of the cluster to analyze')
            [CompletionResult]::new('--cluster', '--cluster', [CompletionResultType]::ParameterName, 'The name of the cluster to analyze')
            [CompletionResult]::new('-r', '-r', [CompletionResultType]::ParameterName, 'The AWS region where the cluster is provisioned')
            [CompletionResult]::new('--region', '--region', [CompletionResultType]::ParameterName, 'The AWS region where the cluster is provisioned')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'The AWS profile to use to access the cluster')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'The AWS profile to use to access the cluster')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'f')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'format')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Write to file instead of stdout')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Write to file instead of stdout')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1')
            [CompletionResult]::new('--target-version', '--target-version', [CompletionResultType]::ParameterName, 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to an eksup configuration file (default: .eksup.yaml in cwd)')
            [CompletionResult]::new('--ignore-recommended', '--ignore-recommended', [CompletionResultType]::ParameterName, 'Exclude recommendations from the output')
            [CompletionResult]::new('--show-suppressed', '--show-suppressed', [CompletionResultType]::ParameterName, 'Include findings suppressed by .eksup.yaml ignore rules')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'eksup;create' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('playbook', 'playbook', [CompletionResultType]::ParameterValue, 'Create a playbook for upgrading an Amazon EKS cluster')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'eksup;create;playbook' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'The name of the cluster to analyze')
            [CompletionResult]::new('--cluster', '--cluster', [CompletionResultType]::ParameterName, 'The name of the cluster to analyze')
            [CompletionResult]::new('-r', '-r', [CompletionResultType]::ParameterName, 'The AWS region where the cluster is provisioned')
            [CompletionResult]::new('--region', '--region', [CompletionResultType]::ParameterName, 'The AWS region where the cluster is provisioned')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'The AWS profile to use to access the cluster')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'The AWS profile to use to access the cluster')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Name of the playbook saved locally')
            [CompletionResult]::new('--filename', '--filename', [CompletionResultType]::ParameterName, 'Name of the playbook saved locally')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1')
            [CompletionResult]::new('--target-version', '--target-version', [CompletionResultType]::ParameterName, 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to an eksup configuration file (default: .eksup.yaml in cwd)')
            [CompletionResult]::new('--ignore-recommended', '--ignore-recommended', [CompletionResultType]::ParameterName, 'Exclude recommendations from the output')
            [CompletionResult]::new('--show-suppressed', '--show-suppressed', [CompletionResultType]::ParameterName, 'Include findings suppressed by .eksup.yaml ignore rules')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'eksup;create;help' {
            [CompletionResult]::new('playbook', 'playbook', [CompletionResultType]::ParameterValue, 'Create a playbook for upgrading an Amazon EKS cluster')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'eksup;create;help;playbook' {
            break
        }
        'eksup;create;help;help' {
            break
        }
        'eksup;completion' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'eksup;man' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Increase logging verbosity')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Decrease logging verbosity')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'eksup;help' {
            [CompletionResult]::new('analyze', 'analyze', [CompletionResultType]::ParameterValue, 'Analyze an Amazon EKS cluster for potential upgrade issues')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create artifacts using the analysis data')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion script for the given shell')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Generate the man page for eksup')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'eksup;help;analyze' {
            break
        }
        'eksup;help;create' {
            [CompletionResult]::new('playbook', 'playbook', [CompletionResultType]::ParameterValue, 'Create a playbook for upgrading an Amazon EKS cluster')
            break
        }
        'eksup;help;create;playbook' {
            break
        }
        'eksup;help;completion' {
            break
        }
        'eksup;help;man' {
            break
        }
        'eksup;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
