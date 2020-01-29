defmodule Thulani.Bot.Config do
  @env_vars %{
    database_url: nil,
    spreadsheet_id: nil,
    sheets_api_key: nil,
    steam_api_key: nil,
    max_sheet_column: "zz",
    default_hist: "5",
    max_hist: "30"
  }

  require Logger

  def init! do
    load_env()
    |> Enum.each(fn {application, vals} ->
      Enum.each(vals, fn {key, val} -> Application.put_env(application, key, val) end)
    end)

    if System.get_env("THULANI_DEBUG") do
      Application.put_env(:logger, :level, :debug)
    end
  end

  def load_env do
    %{
      nostrum: [
        token: System.fetch_env!("THULANI_TOKEN"),
        shards: System.get_env("THULANI_DISCORD_SHARDS", "1") |> Integer.parse()
      ],
      thulani: thulani_env()
    }
  end

  defp thulani_env do
    @env_vars
    |> Enum.map(fn {env_var, default} ->
      canonical_env_var =
        env_var
        |> to_string
        |> String.upcase()
        |> (fn x -> "THULANI_" <> x end).()

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
