
use builtin;
use str;

set edit:completion:arg-completer[eksup] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'eksup'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'eksup'= {
            cand -v 'Increase logging verbosity'
            cand --verbose 'Increase logging verbosity'
            cand -q 'Decrease logging verbosity'
            cand --quiet 'Decrease logging verbosity'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
            cand analyze 'Analyze an Amazon EKS cluster for potential upgrade issues'
            cand create 'Create artifacts using the analysis data'
            cand completion 'Generate shell completion script for the given shell'
            cand man 'Generate the man page for eksup'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'eksup;analyze'= {
            cand -c 'The name of the cluster to analyze'
            cand --cluster 'The name of the cluster to analyze'
            cand -r 'The AWS region where the cluster is provisioned'
            cand --region 'The AWS region where the cluster is provisioned'
            cand -p 'The AWS profile to use to access the cluster'
            cand --profile 'The AWS profile to use to access the cluster'
            cand -f 'f'
            cand --format 'format'
            cand -o 'Write to file instead of stdout'
            cand --output 'Write to file instead of stdout'
            cand -t 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1'
            cand --target-version 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1'
            cand --config 'Path to an eksup configuration file (default: .eksup.yaml in cwd)'
            cand --ignore-recommended 'Exclude recommendations from the output'
            cand --show-suppressed 'Include findings suppressed by .eksup.yaml ignore rules'
            cand -v 'Increase logging verbosity'
            cand --verbose 'Increase logging verbosity'
            cand -q 'Decrease logging verbosity'
            cand --quiet 'Decrease logging verbosity'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'eksup;create'= {
            cand -v 'Increase logging verbosity'
            cand --verbose 'Increase logging verbosity'
            cand -q 'Decrease logging verbosity'
            cand --quiet 'Decrease logging verbosity'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
            cand playbook 'Create a playbook for upgrading an Amazon EKS cluster'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'eksup;create;playbook'= {
            cand -c 'The name of the cluster to analyze'
            cand --cluster 'The name of the cluster to analyze'
            cand -r 'The AWS region where the cluster is provisioned'
            cand --region 'The AWS region where the cluster is provisioned'
            cand -p 'The AWS profile to use to access the cluster'
            cand --profile 'The AWS profile to use to access the cluster'
            cand -f 'Name of the playbook saved locally'
            cand --filename 'Name of the playbook saved locally'
            cand -t 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1'
            cand --target-version 'Target Kubernetes version for the upgrade (e.g. "1.34"). Defaults to current + 1'
            cand --config 'Path to an eksup configuration file (default: .eksup.yaml in cwd)'
            cand --ignore-recommended 'Exclude recommendations from the output'
            cand --show-suppressed 'Include findings suppressed by .eksup.yaml ignore rules'
            cand -v 'Increase logging verbosity'
            cand --verbose 'Increase logging verbosity'
            cand -q 'Decrease logging verbosity'
            cand --quiet 'Decrease logging verbosity'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'eksup;create;help'= {
            cand playbook 'Create a playbook for upgrading an Amazon EKS cluster'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'eksup;create;help;playbook'= {
        }
        &'eksup;create;help;help'= {
        }
        &'eksup;completion'= {
            cand -v 'Increase logging verbosity'
            cand --verbose 'Increase logging verbosity'
            cand -q 'Decrease logging verbosity'
            cand --quiet 'Decrease logging verbosity'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'eksup;man'= {
            cand -v 'Increase logging verbosity'
            cand --verbose 'Increase logging verbosity'
            cand -q 'Decrease logging verbosity'
            cand --quiet 'Decrease logging verbosity'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'eksup;help'= {
            cand analyze 'Analyze an Amazon EKS cluster for potential upgrade issues'
            cand create 'Create artifacts using the analysis data'
            cand completion 'Generate shell completion script for the given shell'
            cand man 'Generate the man page for eksup'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'eksup;help;analyze'= {
        }
        &'eksup;help;create'= {
            cand playbook 'Create a playbook for upgrading an Amazon EKS cluster'
        }
        &'eksup;help;create;playbook'= {
        }
        &'eksup;help;completion'= {
        }
        &'eksup;help;man'= {
        }
        &'eksup;help;help'= {
        }
    ]
    $completions[$command]
}
