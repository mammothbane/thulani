defmodule Thulani.Bot.Application do
  use Application

  alias Thulani.Bot.Config

  @applications [
    :nostrum
  ]

  def start(_type, _args) do
    Config.init!()
    Enum.each(@applications, fn a -> {:ok, _} = Application.ensure_all_started(a) end)

    children = [
      Thulani.Bot.Supervisor
    ]

    Supervisor.start_link(children, strategy: :one_for_one)
  end
end
