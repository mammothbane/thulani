defmodule Thulani.Bot.EnvConfigProvider do
  @behaviour Config.Provider

  @env_vars %{
    database_url: nil,
    spreadsheet_id: nil,
    sheets_api_key: nil,
    steam_api_key: nil,
    max_sheet_column: "zz",
    default_hist: "5",
    max_hist: "30"
  }

  def init(prefix) do
    "#{String.upcase(prefix)}_"
  end

  def load(config, prefix) do
    logger_config =
      if System.get_env("#{prefix}DEBUG") do
        %{
          logger: [level: :debug]
        }
      else
        %{}
      end

    result =
      %{
        nostrum: [
          token: System.fetch_env!("#{prefix}TOKEN"),
          shards: System.get_env("#{prefix}DISCORD_SHARDS", "1") |> Integer.parse()
        ],
        thulani_bot: thulani_env(prefix)
      }
      |> Map.merge(logger_config)

    IO.inspect(Config.Reader.merge(config, result))
  end

  defp thulani_env(prefix) do
    @env_vars
    |> Enum.map(fn {env_var, default} ->
      canonical_env_var =
        env_var
        |> to_string
        |> String.upcase()
        |> (fn x -> prefix <> x end).()

      value =
        canonical_env_var
        |> System.get_env(default)

      if value == nil do
        raise "required environment variable not found: #{canonical_env_var}"
      end

      {env_var, value}
    end)
  end
end
