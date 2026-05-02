import click
import json
import sys
from pathlib import Path
from typing import Optional

from .tester import PromptTester
from .config import Config
from .reporter import Reporter, ReportFormat


@click.group()
@click.option("--prompts-dir", default="../prompts", help="Path to prompts directory")
@click.option("--schemas-dir", default="../schemas", help="Path to schemas directory")
@click.option("--gateway-url", default="http://127.0.0.1:9000", help="LLM Gateway URL")
@click.option("--verbose", is_flag=True, help="Show detailed output")
@click.option("--json-output", is_flag=True, help="Output results as JSON")
@click.pass_context
def cli(ctx, prompts_dir, schemas_dir, gateway_url, verbose, json_output):
    """Blup prompt-tester - validate prompt templates against fixtures."""
    ctx.ensure_object(dict)
    ctx.obj["config"] = Config(
        prompts_dir=prompts_dir,
        schemas_dir=schemas_dir,
        gateway_url=gateway_url,
        verbose=verbose,
        json_output=json_output,
    )


@cli.command()
@click.option("--gateway", is_flag=True, help="Use real LLM Gateway instead of mocks")
@click.pass_context
def test_all(ctx, gateway):
    """Test all prompt templates against their fixtures."""
    config = ctx.obj["config"]
    config.use_gateway = gateway

    tester = PromptTester(config)
    results = tester.test_all()

    reporter = Reporter(
        format=ReportFormat.JSON if config.json_output else ReportFormat.TERMINAL
    )
    reporter.print(results)

    if not results.all_passed:
        sys.exit(1)


@cli.command()
@click.argument("prompt_name")
@click.option("--gateway", is_flag=True, help="Use real LLM Gateway")
@click.pass_context
def test(ctx, prompt_name, gateway):
    """Test a specific prompt template."""
    config = ctx.obj["config"]
    config.use_gateway = gateway

    tester = PromptTester(config)
    results = tester.test_prompt(prompt_name)

    reporter = Reporter(
        format=ReportFormat.JSON if config.json_output else ReportFormat.TERMINAL
    )
    reporter.print_prompt_results(prompt_name, results)

    if not all(r.passed for r in results):
        sys.exit(1)


@cli.command()
@click.argument("prompt_name")
@click.option(
    "--update-fixtures", is_flag=True, help="Update fixtures with captured responses"
)
@click.pass_context
def capture(ctx, prompt_name, update_fixtures):
    """Capture LLM Gateway responses as fixtures for a prompt."""
    config = ctx.obj["config"]
    config.use_gateway = True

    tester = PromptTester(config)
    fixture = tester.capture(prompt_name)

    if update_fixtures:
        tester.save_fixture(prompt_name, fixture)
        click.echo(f"Fixture saved for {prompt_name}")
    else:
        click.echo(json.dumps(fixture, indent=2))


@cli.command()
@click.pass_context
def list(ctx):
    """List all prompts and their test status."""
    config = ctx.obj["config"]
    tester = PromptTester(config)
    prompts = tester.list_prompts()

    for prompt_name, fixture_count in prompts.items():
        click.echo(f"{prompt_name}: {fixture_count} fixtures")


@cli.command()
@click.argument("prompt_name")
@click.pass_context
def gen_fixtures(ctx, prompt_name):
    """Generate fixture scaffolding for a prompt."""
    config = ctx.obj["config"]
    tester = PromptTester(config)
    tester.generate_fixture_scaffolding(prompt_name)
    click.echo(f"Generated fixture scaffolding for {prompt_name}")


if __name__ == "__main__":
    cli()
