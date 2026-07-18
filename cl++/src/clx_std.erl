-module(clx_std).

-export([to_boolean/1, print/1, random/2]).

to_boolean(true) ->
    true;
to_boolean(false) ->
    false;
to_boolean(0) ->
    false;
to_boolean(-0.0) ->
    false;
to_boolean(+0.0) ->
    false;
to_boolean([]) ->
    false;
to_boolean(<<>>) ->
    false;
to_boolean(undefined) ->
    false;
to_boolean(_) ->
    true.

print(String) when is_list(String) ->
    io:format("~s~n", [String]);
print(Other) ->
    io:format("~p~n", [Other]).

random(Min, Max) when Min =< Max ->
    Time = abs(erlang:system_time()),

    (Time rem (Max - Min + 1)) + Min.
