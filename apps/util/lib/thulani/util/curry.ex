defmodule Thulani.Util.Curry do
  @moduledoc false

  def curry(f) do
    {_, arity} = :erlang.fun_info(f, :arity)
    curry(f, arity, [])
  end

  defp curry(f, 0, args) do
    apply(f, Enum.reverse(args))
  end

  defp curry(f, arity, args) do
    fn arg -> curry(f, arity - 1, [arg | args]) end
  end
end
