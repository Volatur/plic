-module(stingray_runtime).
-export([flow_run/2, str_append/2,
         tcp_listen/2, tcp_accept/1, tcp_connect/5, tcp_recv/2, tcp_send/2, tcp_close/1,
         get_line/0]).

%% #flow:N# — spawn N parallel copies of Fun, wait for all to finish
flow_run(N, Fun) ->
    Self = self(),
    Pids = [spawn(fun() ->
        Fun(),
        Self ! {flow_done, self()}
    end) || _ <- lists:seq(1, N)],
    Monitors = [monitor(process, P) || P <- Pids],
    flow_wait(Monitors, length(Pids), 0, 10000).

flow_wait(_Monitors, N, N, _Timeout) -> ok;
flow_wait(Monitors, Total, Done, Timeout) ->
    receive
        {flow_done, _} -> flow_wait(Monitors, Total, Done + 1, Timeout);
        {'DOWN', _Ref, process, _Pid, _Reason} -> flow_wait(Monitors, Total, Done + 1, Timeout)
    after Timeout -> ok
    end.

%% String-aware + operator
str_append(A, B) when is_list(A), is_list(B) -> A ++ B;
str_append(A, B) when is_list(A), is_number(B) -> A ++ erlang:integer_to_list(B);
str_append(A, B) when is_number(A), is_list(B) -> erlang:integer_to_list(A) ++ B;
str_append(A, B) when is_number(A), is_number(B) -> A + B;
str_append(A, B) -> lists:flatten(io_lib:format("~p~p", [A, B])).

%% TCP helpers — create Erlang tuples that Stingray can't construct directly
tcp_listen(Port, _Opts) ->
    case gen_tcp:listen(Port, [binary, {active, false}, {reuseaddr, true}]) of
        {ok, Sock} -> Sock;
        {error, _} -> false
    end.

tcp_accept(ListenSock) ->
    case gen_tcp:accept(ListenSock) of
        {ok, Sock} -> Sock;
        {error, _} -> false
    end.

tcp_connect(A, B, C, D, Port) ->
    case gen_tcp:connect({A, B, C, D}, Port, [binary, {active, false}]) of
        {ok, Sock} -> Sock;
        {error, _} -> false
    end.

tcp_recv(Sock, Timeout) ->
    case gen_tcp:recv(Sock, 0, Timeout) of
        {ok, Data} -> binary_to_list(Data);
        {error, _} -> false
    end.

tcp_send(Sock, Msg) ->
    case gen_tcp:send(Sock, Msg) of
        ok -> true;
        {error, _} -> false
    end.

tcp_close(Sock) ->
    gen_tcp:close(Sock).

%% Read a line from stdin
get_line() ->
    case io:get_line("") of
        eof -> false;
        Line -> string:trim(Line, both, "\n")
    end.
