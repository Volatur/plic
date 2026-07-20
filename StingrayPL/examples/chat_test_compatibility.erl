-module(chat_test_compatibility).
-export([run/0]).

%% Comprehensive compatibility test: Stingray server <-> Stingray client
run() ->
    code:add_patha("src"),
    code:add_patha("examples"),
    io:format("========================================~n"),
    io:format("  STINGRAY CLIENT-SERVER COMPAT TEST~n"),
    io:format("========================================~n~n"),

    %% Start server in background
    spawn(fun() -> chat_server:main() end),
    timer:sleep(1500),

    %% Test 1: Basic connection
    io:format("[TEST 1] Basic connection...~n"),
    test_connect(),

    %% Test 2: Send and receive echo
    io:format("[TEST 2] Send + receive echo...~n"),
    test_echo(),

    %% Test 3: Multiple messages
    io:format("[TEST 3] Multiple messages...~n"),
    test_multi_msg(),

    %% Test 4: Two clients
    io:format("[TEST 4] Two clients...~n"),
    test_two_clients(),

    %% Test 5: Client disconnect
    io:format("[TEST 5] Client disconnect...~n"),
    test_disconnect(),

    io:format("~n========================================~n"),
    io:format("  ALL COMPAT TESTS PASSED~n"),
    io:format("========================================~n").

test_connect() ->
    case gen_tcp:connect({127,0,0,1}, 9999, [binary, {active, false}]) of
        {ok, Sock} ->
            io:format("  Connected OK~n"),
            gen_tcp:close(Sock);
        {error, E} ->
            io:format("  FAILED: ~p~n", [E]),
            halt(1)
    end.

test_echo() ->
    {ok, S} = gen_tcp:connect({127,0,0,1}, 9999, [binary, {active, false}]),
    gen_tcp:send(S, <<"Hello from test!\n">>),
    {ok, D} = gen_tcp:recv(S, 0, 3000),
    R = binary_to_list(D),
    case lists:prefix("echo: Hello from test!", R) of
        true -> io:format("  Echo OK~n");
        false -> io:format("  FAILED: ~p~n", [R]), halt(1)
    end,
    gen_tcp:close(S).

test_multi_msg() ->
    {ok, S} = gen_tcp:connect({127,0,0,1}, 9999, [binary, {active, false}]),
    lists:foreach(fun(N) ->
        Msg = "msg " ++ integer_to_list(N) ++ "\n",
        gen_tcp:send(S, list_to_binary(Msg)),
        {ok, D} = gen_tcp:recv(S, 0, 3000),
        case lists:prefix("echo: ", binary_to_list(D)) of
            true -> ok;
            false -> io:format("  FAILED at msg ~p~n", [N]), halt(1)
        end
    end, [1,2,3]),
    io:format("  3 messages echoed OK~n"),
    gen_tcp:close(S).

test_two_clients() ->
    {ok, C1} = gen_tcp:connect({127,0,0,1}, 9999, [binary, {active, false}]),
    gen_tcp:send(C1, <<"client1 msg\n">>),
    {ok, D1} = gen_tcp:recv(C1, 0, 3000),
    R1 = binary_to_list(D1),
    gen_tcp:close(C1),

    {ok, C2} = gen_tcp:connect({127,0,0,1}, 9999, [binary, {active, false}]),
    gen_tcp:send(C2, <<"client2 msg\n">>),
    {ok, D2} = gen_tcp:recv(C2, 0, 3000),
    R2 = binary_to_list(D2),
    gen_tcp:close(C2),

    case {lists:prefix("echo: ", R1), lists:prefix("echo: ", R2)} of
        {true, true} -> io:format("  Both clients echo OK~n");
        _ -> io:format("  FAILED: ~p, ~p~n", [R1, R2]), halt(1)
    end.

test_disconnect() ->
    {ok, S} = gen_tcp:connect({127,0,0,1}, 9999, [binary, {active, false}]),
    gen_tcp:close(S),
    timer:sleep(500),
    io:format("  Disconnect OK~n").
