defmodule Thulani.Bot.Application do
  alias Thulani.Bot.Config
  use Application

  def start(_type, _args) do
    Config.init!()
    Application.start(:nostrum)

    children = []

    opts = [strategy: :one_for_one, name: Thulani.Bot.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
