defmodule Thulani.Bot.MixProject do
  use Mix.Project

  def project do
    [
      app: :thulani_bot,
      version: "0.1.0",
      build_path: "../../_build",
      config_path: "../../config/config.exs",
      deps_path: "../../deps",
      lockfile: "../../mix.lock",
      elixir: "~> 1.9",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {Thulani.Bot.Application, []}
    ]
  end

  defp deps do
    [
      {:nostrum, "~> 0.4", runtime: false},
      {:util, in_umbrella: true}
    ]
  end
end
