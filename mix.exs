defmodule Thulani.MixProject do
  use Mix.Project

  def project do
    [
      apps_path: "apps",
      version: "0.1.0",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      releases: releases(),
      default_release: :thulani_flex
    ]
  end

  defp deps do
    [
      {:toml, "~> 0.6.1"}
    ]
  end

  defp releases do
    config_providers = [
      #      {Toml.Provider, ["./config.toml", "/etc/thulani/config.toml"]}
    ]

    applications = [thulani_bot: :permanent]

    [
      thulani_flex: [
        include_executables_for: [:unix, :windows],
        strip_beams: false,
        config_providers: config_providers,
        applications: applications
      ],
      thulani_prod: [
        include_executables_for: [:unix],
        strip_beams: true,
        config_providers: config_providers,
        applications: applications
      ]
    ]
  end
end
