-module(stingray_lexer).
-export([tokenize/1]).

-type token() :: {atom(), binary() | number(), pos_integer(), pos_integer()}.

-spec tokenize(binary()) -> [token()] | {error, term()}.
tokenize(Input) when is_binary(Input) ->
    case scan(Input, 1, 1, []) of
        {ok, Tokens} -> lists:reverse(Tokens);
        {error, _} = Err -> Err
    end.

%% ============================================================================
%% Main scan loop
%% ============================================================================
scan(<<>>, _Line, _Col, Acc) -> {ok, Acc};
scan(<<$\n, Rest/binary>>, Line, _Col, Acc) -> scan(Rest, Line + 1, 1, Acc);
scan(<<$\s, Rest/binary>>, Line, Col, Acc) -> scan(Rest, Line, Col + 1, Acc);
scan(<<$\t, Rest/binary>>, Line, Col, Acc) -> scan(Rest, Line, Col + 4, Acc);
scan(<<$\r, Rest/binary>>, Line, Col, Acc) -> scan(Rest, Line, Col + 1, Acc);

scan(<<$/, $/, Rest/binary>>, Line, Col, Acc) -> scan_line_comment(Rest, Line, Col + 2, Acc);
scan(<<$/, $*, Rest/binary>>, Line, Col, Acc) -> scan_block_comment(Rest, Line, Col + 2, Acc);
scan(<<$#, Rest/binary>>, Line, Col, Acc) -> scan_directive(Rest, Line, Col + 1, Acc);
scan(<<$", Rest/binary>>, Line, Col, Acc) ->
    scan_string(Rest, Line, Col + 1, Line, Col, <<>>, Acc);
scan(<<$', Rest/binary>>, Line, Col, Acc) ->
    scan_char(Rest, Line, Col + 1, Line, Col, <<>>, Acc);

scan(<<C, _/binary>> = Input, Line, Col, Acc) when C >= $0, C =< $9 ->
    scan_number(Input, Line, Col, Acc);

%% Float with leading dot: .5, .123e10
scan(<<$., C, Rest/binary>>, Line, Col, Acc) when C >= $0, C =< $9 ->
    scan_float_frac(<<C, Rest/binary>>, Line, Col + 2, <<>>, Col, Acc);

scan(<<C, _/binary>> = Input, Line, Col, Acc) when
        (C >= $a andalso C =< $z) orelse
        (C >= $A andalso C =< $Z) orelse C =:= $_ ->
    scan_identifier(Input, Line, Col, <<>>, Acc);

scan(<<$=, $=, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 2, [{equal_equal, <<"==">>, Line, Col} | Acc]);
scan(<<$!, $=, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 2, [{bang_equal, <<"!=">>, Line, Col} | Acc]);
scan(<<$<, $=, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 2, [{less_equal, <<"<=">>, Line, Col} | Acc]);
scan(<<$>, $=, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 2, [{greater_equal, <<">=">>, Line, Col} | Acc]);
scan(<<$&, $&, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 2, [{and_and, <<"&&">>, Line, Col} | Acc]);
scan(<<$+, $+, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 2, [{plus_plus, <<"++">>, Line, Col} | Acc]);
scan(<<$|, $|, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 2, [{pipe_pipe, <<"||">>, Line, Col} | Acc]);

scan(<<C, Rest/binary>>, Line, Col, Acc) ->
    case single_char_token(C) of
        {ok, Type, Sym} -> scan(Rest, Line, Col + 1, [{Type, Sym, Line, Col} | Acc]);
        error -> {error, {Line, Col, {unexpected_char, C}}}
    end.

single_char_token($=)  -> {ok, equal,       <<"=">>};
single_char_token($!)  -> {ok, bang,        <<"!">>};
single_char_token($+)  -> {ok, plus,        <<"+">>};
single_char_token($*)  -> {ok, star,        <<"*">>};
single_char_token($/)  -> {ok, slash,       <<"/">>};
single_char_token($%)  -> {ok, percent,     <<"%">>};
single_char_token($<)  -> {ok, less,        <<"<">>};
single_char_token($>)  -> {ok, greater,     <<">">>};
single_char_token($&)  -> {ok, ampersand,   <<"&">>};
single_char_token($|)  -> {ok, pipe,        <<"|">>};
single_char_token($-)  -> {ok, minus,       <<"-">>};
single_char_token($.)  -> {ok, dot,         <<".">>};
single_char_token($,)  -> {ok, comma,       <<",">>};
single_char_token($:)  -> {ok, colon,       <<":">>};
single_char_token($()  -> {ok, lparen,      <<"(">>};
single_char_token($))  -> {ok, rparen,      <<")">>};
single_char_token(${)  -> {ok, lbrace,      <<"{">>};
single_char_token($})  -> {ok, rbrace,      <<"}">>};
single_char_token($[)  -> {ok, lbracket,    <<"[">>};
single_char_token($])  -> {ok, rbracket,    <<"]">>};
single_char_token(_)   -> error.

%% ============================================================================
%% Comments
%% ============================================================================
scan_line_comment(<<$\n, Rest/binary>>, Line, _Col, Acc) -> scan(Rest, Line + 1, 1, Acc);
scan_line_comment(<<>>, _Line, _Col, Acc) -> {ok, Acc};
scan_line_comment(<<_/utf8, Rest/binary>>, Line, Col, Acc) -> scan_line_comment(Rest, Line, Col + 1, Acc).

scan_block_comment(<<>>, Line, Col, _Acc) -> {error, {Line, Col, unterminated_comment}};
scan_block_comment(<<$\n, Rest/binary>>, Line, _Col, Acc) -> scan_block_comment(Rest, Line + 1, 1, Acc);
scan_block_comment(<<$*, $/, Rest/binary>>, Line, Col, Acc) -> scan(Rest, Line, Col + 2, Acc);
scan_block_comment(<<_/utf8, Rest/binary>>, Line, Col, Acc) -> scan_block_comment(Rest, Line, Col + 1, Acc).

%% ============================================================================
%% Compiler directives
%% ============================================================================
scan_directive(<<$u, $s, $e, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 3, [{directive_use, <<"use">>, Line, Col - 1} | Acc]);
scan_directive(<<$s, $i, $d, $e, $w, $a, $y, $#, Rest/binary>>, Line, Col, Acc) ->
    scan(Rest, Line, Col + 8, [{directive_sideway, <<"sideway">>, Line, Col - 1} | Acc]);
scan_directive(<<$f, $l, $o, $w, $:, Rest/binary>>, Line, Col, Acc) ->
    scan_flow_value(Rest, Line, Col + 5, Acc);
scan_directive(<<C, _/binary>>, Line, Col, _Acc) ->
    {error, {Line, Col - 1, {unexpected_directive_char, C}}}.

scan_flow_value(Input, Line, Col, Acc) ->
    case scan_flow_number(Input, Line, Col, <<>>) of
        {ok, NumBin, Rest, NewLine, NewCol} ->
            case Rest of
                <<$#, Rest2/binary>> ->
                    N = binary_to_integer(NumBin),
                    scan(Rest2, NewLine, NewCol + 1, [{directive_flow, N, Line, Col - 6} | Acc]);
                _ -> {error, {NewLine, NewCol, missing_flow_hash}}
            end;
        {error, _} = Err -> Err
    end.

scan_flow_number(<<C, Rest/binary>>, Line, Col, Acc) when C >= $0, C =< $9 ->
    scan_flow_number(Rest, Line, Col + 1, <<Acc/binary, C>>);
scan_flow_number(<<>>, _Line, _Col, _Acc) -> {error, {_Line, _Col, unterminated_directive}};
scan_flow_number(Rest, Line, Col, Acc) when byte_size(Acc) > 0 -> {ok, Acc, Rest, Line, Col};
scan_flow_number(<<C, _/binary>>, Line, Col, <<>>) -> {error, {Line, Col, {expected_digit, C}}}.

%% ============================================================================
%% Strings
%% ============================================================================
scan_string(<<>>, Line, Col, _SL, _SC, _Acc, _OA) -> {error, {Line, Col, unterminated_string}};
scan_string(<<$\\, $n, Rest/binary>>, Line, Col, SL, SC, Acc, OA) ->
    scan_string(Rest, Line, Col + 2, SL, SC, <<Acc/binary, $\n>>, OA);
scan_string(<<$\\, $t, Rest/binary>>, Line, Col, SL, SC, Acc, OA) ->
    scan_string(Rest, Line, Col + 2, SL, SC, <<Acc/binary, $\t>>, OA);
scan_string(<<$\\, $\\, Rest/binary>>, Line, Col, SL, SC, Acc, OA) ->
    scan_string(Rest, Line, Col + 2, SL, SC, <<Acc/binary, $\\>>, OA);
scan_string(<<$\\, $", Rest/binary>>, Line, Col, SL, SC, Acc, OA) ->
    scan_string(Rest, Line, Col + 2, SL, SC, <<Acc/binary, $">>, OA);
scan_string(<<$\\, $0, Rest/binary>>, Line, Col, SL, SC, Acc, OA) ->
    scan_string(Rest, Line, Col + 2, SL, SC, <<Acc/binary, 0>>, OA);
scan_string(<<$\\, C, _/binary>>, Line, Col, _SL, _SC, _Acc, _OA) ->
    {error, {Line, Col, {invalid_escape, C}}};
scan_string(<<$", Rest/binary>>, Line, Col, SL, SC, Acc, OA) ->
    scan(Rest, Line, Col + 1, [{string, Acc, SL, SC} | OA]);
scan_string(<<$\n, Rest/binary>>, Line, _Col, SL, SC, Acc, OA) ->
    scan_string(Rest, Line + 1, 1, SL, SC, <<Acc/binary, $\n>>, OA);
scan_string(<<C/utf8, Rest/binary>>, Line, Col, SL, SC, Acc, OA) ->
    scan_string(Rest, Line, Col + 1, SL, SC, <<Acc/binary, C/utf8>>, OA).

%% ============================================================================
%% Chars
%% ============================================================================
scan_char(<<>>, Line, Col, _SL, _SC, _CA, _OA) -> {error, {Line, Col, unterminated_char}};
scan_char(<<$\\, $n, $', Rest/binary>>, Line, Col, SL, SC, _CA, OA) ->
    scan(Rest, Line, Col + 3, [{char, $\n, SL, SC} | OA]);
scan_char(<<$\\, $t, $', Rest/binary>>, Line, Col, SL, SC, _CA, OA) ->
    scan(Rest, Line, Col + 3, [{char, $\t, SL, SC} | OA]);
scan_char(<<$\\, $\\, $', Rest/binary>>, Line, Col, SL, SC, _CA, OA) ->
    scan(Rest, Line, Col + 3, [{char, $\\, SL, SC} | OA]);
scan_char(<<$\\, $', $', Rest/binary>>, Line, Col, SL, SC, _CA, OA) ->
    scan(Rest, Line, Col + 3, [{char, $', SL, SC} | OA]);
scan_char(<<$\\, $0, $', Rest/binary>>, Line, Col, SL, SC, _CA, OA) ->
    scan(Rest, Line, Col + 3, [{char, 0, SL, SC} | OA]);
scan_char(<<$\\, C, _/binary>>, Line, Col, _SL, _SC, _CA, _OA) ->
    {error, {Line, Col, {invalid_escape, C}}};
scan_char(<<$', _/binary>>, Line, Col, _SL, _SC, _CA, _OA) ->
    {error, {Line, Col - 1, empty_char}};
scan_char(<<C/utf8, $', Rest/binary>>, Line, Col, SL, SC, _CA, OA) ->
    scan(Rest, Line, Col + 2, [{char, C, SL, SC} | OA]);
scan_char(<<C/utf8, Rest/binary>>, Line, Col, SL, SC, CA, OA) ->
    scan_char(Rest, Line, Col + 1, SL, SC, <<CA/binary, C/utf8>>, OA).

%% ============================================================================
%% Numbers
%% ============================================================================
scan_number(<<$0, $x, Rest/binary>>, Line, Col, Acc) -> scan_hex(Rest, Line, Col + 2, <<>>, Acc);
scan_number(<<$0, $b, Rest/binary>>, Line, Col, Acc) -> scan_bin(Rest, Line, Col + 2, <<>>, Acc);
scan_number(<<$0, $o, Rest/binary>>, Line, Col, Acc) -> scan_oct(Rest, Line, Col + 2, <<>>, Acc);
scan_number(Input, Line, Col, Acc) -> scan_decimal(Input, Line, Col, <<>>, Col, Acc).

scan_hex(<<C, Rest/binary>>, Line, Col, Acc, OA) when
        (C >= $0 andalso C =< $9) orelse (C >= $a andalso C =< $f) orelse
        (C >= $A andalso C =< $F) orelse C =:= $_ ->
    scan_hex(Rest, Line, Col + 1, <<Acc/binary, C>>, OA);
scan_hex(_, _Line, _Col, <<>>, _OA) -> {error, {_Line, _Col - 1, empty_hex_literal}};
scan_hex(Rest, Line, Col, Acc, OA) ->
    Clean = remove_underscores(Acc),
    Val = list_to_integer(binary_to_list(Clean), 16),
    T = {integer, Val, Line, Col - byte_size(Acc) - 2},
    scan(Rest, Line, Col, [T | OA]).

scan_bin(<<C, Rest/binary>>, Line, Col, Acc, OA) when
        C =:= $0 orelse C =:= $1 orelse C =:= $_ ->
    scan_bin(Rest, Line, Col + 1, <<Acc/binary, C>>, OA);
scan_bin(_, _Line, _Col, <<>>, _OA) -> {error, {_Line, _Col - 1, empty_bin_literal}};
scan_bin(Rest, Line, Col, Acc, OA) ->
    Clean = remove_underscores(Acc),
    Val = list_to_integer(binary_to_list(Clean), 2),
    T = {integer, Val, Line, Col - byte_size(Acc) - 2},
    scan(Rest, Line, Col, [T | OA]).

scan_oct(<<C, Rest/binary>>, Line, Col, Acc, OA) when
        (C >= $0 andalso C =< $7) orelse C =:= $_ ->
    scan_oct(Rest, Line, Col + 1, <<Acc/binary, C>>, OA);
scan_oct(_, _Line, _Col, <<>>, _OA) -> {error, {_Line, _Col - 1, empty_oct_literal}};
scan_oct(Rest, Line, Col, Acc, OA) ->
    Clean = remove_underscores(Acc),
    Val = list_to_integer(binary_to_list(Clean), 8),
    T = {integer, Val, Line, Col - byte_size(Acc) - 2},
    scan(Rest, Line, Col, [T | OA]).

%% ============================================================================
%% Floats (6 params: Rest/Next, Line, Col, Accumulator, StartCol, OuterAcc)
%% ============================================================================
%% StartCol (SC) = column where the float literal begins in source.

scan_decimal(<<C, Rest/binary>>, Line, Col, Acc, SC, OA) when C >= $0, C =< $9 ->
    scan_decimal(Rest, Line, Col + 1, <<Acc/binary, C>>, SC, OA);
scan_decimal(<<$_, C, Rest/binary>>, Line, Col, Acc, SC, OA) when C >= $0, C =< $9 ->
    scan_decimal(Rest, Line, Col + 2, <<Acc/binary, $_, C>>, SC, OA);
scan_decimal(<<$., Rest/binary>>, Line, Col, Acc, SC, OA) ->
    scan_float_frac(Rest, Line, Col + 1, Acc, SC, OA);
scan_decimal(<<C, _/binary>> = Input, Line, Col, Acc, SC, OA) when C =:= $e; C =:= $E ->
    scan_float_exp(Input, Line, Col, <<Acc/binary>>, SC, OA);
scan_decimal(Rest, Line, Col, Acc, _SC, OA) when byte_size(Acc) > 0 ->
    Clean = remove_underscores(Acc),
    Val = binary_to_integer(Clean),
    T = {integer, Val, Line, Col - byte_size(Acc)},
    scan(Rest, Line, Col, [T | OA]);
scan_decimal(_, Line, Col, <<>>, _SC, _OA) -> {error, {Line, Col, unexpected_number_start}}.

scan_float_frac(<<C, Rest/binary>>, Line, Col, IntAcc, SC, OA) when C >= $0, C =< $9 ->
    scan_float_frac_digits(Rest, Line, Col + 1, IntAcc, <<C>>, SC, OA);
scan_float_frac(<<C, _/binary>> = Input, Line, Col, IntAcc, SC, OA) when C =:= $e; C =:= $E ->
    scan_float_exp(Input, Line, Col, <<IntAcc/binary, $.>>, SC, OA);
scan_float_frac(Rest, Line, Col, IntAcc, SC, OA) ->
    FloatBin = make_float_bin(<<IntAcc/binary, $.>>),
    Val = binary_to_float(FloatBin),
    T = {float, Val, 1, SC},
    scan(Rest, Line, Col, [T | OA]).

scan_float_frac_digits(<<C, Rest/binary>>, Line, Col, IntAcc, FracAcc, SC, OA) when
        C >= $0, C =< $9 ->
    scan_float_frac_digits(Rest, Line, Col + 1, IntAcc, <<FracAcc/binary, C>>, SC, OA);
scan_float_frac_digits(<<C, _/binary>> = Input, Line, Col, IntAcc, FracAcc, SC, OA) when
        C =:= $e; C =:= $E ->
    scan_float_exp(Input, Line, Col, <<IntAcc/binary, $., FracAcc/binary>>, SC, OA);
scan_float_frac_digits(Rest, Line, Col, IntAcc, FracAcc, SC, OA) ->
    FloatBin = make_float_bin(<<IntAcc/binary, $., FracAcc/binary>>),
    Val = binary_to_float(FloatBin),
    T = {float, Val, 1, SC},
    scan(Rest, Line, Col, [T | OA]).

scan_float_exp(<<C, Rest/binary>>, Line, Col, IntAcc, SC, OA) when C =:= $e; C =:= $E ->
    scan_float_exp(Rest, Line, Col + 1, IntAcc, SC, OA);
scan_float_exp(<<$-, Rest/binary>>, Line, Col, IntAcc, SC, OA) ->
    scan_float_exp_digits(Rest, Line, Col + 1, IntAcc, <<$->>, SC, OA);
scan_float_exp(<<$+, Rest/binary>>, Line, Col, IntAcc, SC, OA) ->
    scan_float_exp_digits(Rest, Line, Col + 1, IntAcc, <<$+>>, SC, OA);
scan_float_exp(<<C, Rest/binary>>, Line, Col, IntAcc, SC, OA) when C >= $0, C =< $9 ->
    scan_float_exp_digits(Rest, Line, Col + 1, IntAcc, <<C>>, SC, OA);
scan_float_exp(<<>>, Line, Col, _IntAcc, _SC, _OA) ->
    {error, {Line, Col, unterminated_float_exponent}};
scan_float_exp(<<C, _/binary>>, Line, Col, _IntAcc, _SC, _OA) ->
    {error, {Line, Col, {expected_exp_digit, C}}}.

scan_float_exp_digits(<<C, Rest/binary>>, Line, Col, IntAcc, ExpAcc, SC, OA) when
        C >= $0, C =< $9 ->
    scan_float_exp_digits(Rest, Line, Col + 1, IntAcc, <<ExpAcc/binary, C>>, SC, OA);
scan_float_exp_digits(Rest, Line, Col, IntAcc, ExpAcc, SC, OA) ->
    FloatBin = make_float_bin(<<IntAcc/binary, $e, ExpAcc/binary>>),
    Val = binary_to_float(FloatBin),
    T = {float, Val, 1, SC},
    scan(Rest, Line, Col, [T | OA]).

%% ============================================================================
%% Identifiers and Keywords
%% ============================================================================
scan_identifier(<<C, Rest/binary>>, Line, Col, NameAcc, OuterAcc) when
        (C >= $a andalso C =< $z) orelse (C >= $A andalso C =< $Z) orelse
        (C >= $0 andalso C =< $9) orelse C =:= $_ ->
    scan_identifier(Rest, Line, Col + 1, <<NameAcc/binary, C>>, OuterAcc);
scan_identifier(Rest, Line, Col, NameAcc, OuterAcc) ->
    Keywords = stingray_tokens:keyword_map(),
    Token = case maps:find(NameAcc, Keywords) of
        {ok, Type} -> {Type, NameAcc, Line, Col - byte_size(NameAcc)};
        error ->
            case is_upper_start(NameAcc) of
                true  -> {type_identifier, NameAcc, Line, Col - byte_size(NameAcc)};
                false -> {identifier, NameAcc, Line, Col - byte_size(NameAcc)}
            end
    end,
    scan(Rest, Line, Col, [Token | OuterAcc]).

%% ============================================================================
%% Helpers
%% ============================================================================
remove_underscores(Bin) -> << <<C>> || <<C>> <= Bin, C =/= $_ >>.
is_upper_start(<<C, _/binary>>) when C >= $A, C =< $Z -> true;
is_upper_start(_) -> false.

make_float_bin(Bin) ->
    case binary:match(Bin, <<$.>>) of
        nomatch ->
            ExpPos = case binary:match(Bin, <<$e>>) of
                nomatch -> binary:match(Bin, <<$E>>);
                P -> P
            end,
            case ExpPos of
                nomatch -> <<Bin/binary, ".0">>;
                {Pos, _} ->
                    <<Pre:Pos/binary, Rest/binary>> = Bin,
                    <<Pre/binary, ".0", Rest/binary>>
            end;
        _ ->
            case {binary:last(Bin), binary:at(Bin, 0)} of
                {$., _} -> <<Bin/binary, "0">>;
                {_, $.} -> <<"0", Bin/binary>>;
                _ -> Bin
            end
    end.