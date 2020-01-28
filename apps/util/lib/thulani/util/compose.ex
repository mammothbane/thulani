defmodule Thulani.Util.Compose do
  import Thulani.Util.Curry

  def f <|> g, do: compose(f, g)

  def compose(f, g) when is_function(g) do
    fn arg -> compose(curry(f), curry(g).(arg)) end
  end

  def compose(f, arg) do
    f.(arg)
  end
end
