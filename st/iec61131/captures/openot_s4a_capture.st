FUNCTION_BLOCK OPENOT_CaptureRichWrap
VAR_INPUT
    Execute : BOOL;
END_VAR
VAR_OUTPUT
    Ring : OPENOT_RING256_BUFFER;
    HeadAbs : ULINT;
    OldestAbs : ULINT;
    LostCount : ULINT;
    RunId : ULINT;
    EpochId : ULINT;
    EpochFirstAbs : ULINT;
    DefinitionHash : OPENOT_HASH8;
    PrevDefinitionHash : OPENOT_HASH8;
    Complete : BOOL;
    Error : BOOL;
END_VAR
VAR
    Producer : OPENOT_Producer;
    DefHashOld : OPENOT_HASH8 := [
        BYTE#16#6F, BYTE#16#6C, BYTE#16#64, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#31
    ];
    DefHashNew : OPENOT_HASH8 := [
        BYTE#16#6E, BYTE#16#65, BYTE#16#77, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#32
    ];
    Ran : BOOL;
    ByteIndex : UINT;
    HashIndex : UINT;
END_VAR

IF NOT Ran THEN
    Producer(
        Execute := TRUE,
        Op := UINT#0,
        SourceId := UDINT#10,
        Checkpoint := FALSE,
        DefHashOld := DefHashOld,
        DefHashNew := DefHashNew,
        TransitionEpochId := ULINT#2);
    IF Producer.Error THEN
        Error := TRUE;
    ELSE
        Producer(
            Execute := FALSE,
            Op := UINT#0,
            SourceId := UDINT#10,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#0,
            SourceId := UDINT#10,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#0,
                SourceId := UDINT#10,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#0,
            SourceId := UDINT#10,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#0,
                SourceId := UDINT#10,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#0,
            SourceId := UDINT#20,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#0,
                SourceId := UDINT#20,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#0,
            SourceId := UDINT#20,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#0,
                SourceId := UDINT#20,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#0,
            SourceId := UDINT#10,
            Checkpoint := TRUE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#0,
                SourceId := UDINT#10,
                Checkpoint := TRUE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#0,
            SourceId := UDINT#20,
            Checkpoint := TRUE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#0,
                SourceId := UDINT#20,
                Checkpoint := TRUE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#1,
            SourceId := UDINT#0,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#1,
                SourceId := UDINT#0,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#2,
            SourceId := UDINT#0,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#2,
                SourceId := UDINT#0,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#3,
            SourceId := UDINT#0,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#3,
                SourceId := UDINT#0,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#0,
            SourceId := UDINT#10,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#0,
                SourceId := UDINT#10,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#0,
            SourceId := UDINT#20,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#0,
                SourceId := UDINT#20,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Complete := TRUE;
    END_IF;
    Ran := TRUE;
END_IF;

FOR ByteIndex := UINT#0 TO UINT#255 DO
    Ring[ByteIndex] := Producer.Ring[ByteIndex];
END_FOR;
FOR HashIndex := UINT#0 TO UINT#7 DO
    DefinitionHash[HashIndex] := Producer.DefinitionHash[HashIndex];
    PrevDefinitionHash[HashIndex] := Producer.PrevDefinitionHash[HashIndex];
END_FOR;
HeadAbs := Producer.HeadAbs;
OldestAbs := Producer.OldestAbs;
LostCount := Producer.LostCount;
RunId := Producer.RunId;
EpochId := Producer.EpochId;
EpochFirstAbs := Producer.EpochFirstAbs;
Error := Error OR Producer.RingError;
END_FUNCTION_BLOCK

FUNCTION_BLOCK OPENOT_CaptureLifecycleSurvival
VAR_INPUT
    Execute : BOOL;
END_VAR
VAR_OUTPUT
    Ring : OPENOT_RING256_BUFFER;
    HeadAbs : ULINT;
    OldestAbs : ULINT;
    LostCount : ULINT;
    RunId : ULINT;
    EpochId : ULINT;
    EpochFirstAbs : ULINT;
    DefinitionHash : OPENOT_HASH8;
    PrevDefinitionHash : OPENOT_HASH8;
    Complete : BOOL;
    Error : BOOL;
END_VAR
VAR
    Producer : OPENOT_Producer;
    DefHashOld : OPENOT_HASH8 := [
        BYTE#16#6F, BYTE#16#6C, BYTE#16#64, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#31
    ];
    DefHashNew : OPENOT_HASH8 := [
        BYTE#16#6E, BYTE#16#65, BYTE#16#77, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#32
    ];
    Ran : BOOL;
    ByteIndex : UINT;
    HashIndex : UINT;
END_VAR

IF NOT Ran THEN
    Producer(
        Execute := TRUE,
        Op := UINT#0,
        SourceId := UDINT#21,
        Checkpoint := FALSE,
        DefHashOld := DefHashOld,
        DefHashNew := DefHashNew,
        TransitionEpochId := ULINT#2);
    IF Producer.Error THEN
        Error := TRUE;
    ELSE
        Producer(
            Execute := FALSE,
            Op := UINT#0,
            SourceId := UDINT#21,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#1,
            SourceId := UDINT#0,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#1,
                SourceId := UDINT#0,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#2,
            SourceId := UDINT#0,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#2,
                SourceId := UDINT#0,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Producer(
            Execute := TRUE,
            Op := UINT#3,
            SourceId := UDINT#0,
            Checkpoint := FALSE,
            DefHashOld := DefHashOld,
            DefHashNew := DefHashNew,
            TransitionEpochId := ULINT#2);
        IF Producer.Error THEN
            Error := TRUE;
        ELSE
            Producer(
                Execute := FALSE,
                Op := UINT#3,
                SourceId := UDINT#0,
                Checkpoint := FALSE,
                DefHashOld := DefHashOld,
                DefHashNew := DefHashNew,
                TransitionEpochId := ULINT#2);
        END_IF;
    END_IF;

    IF NOT Error THEN
        Complete := TRUE;
    END_IF;
    Ran := TRUE;
END_IF;

FOR ByteIndex := UINT#0 TO UINT#255 DO
    Ring[ByteIndex] := Producer.Ring[ByteIndex];
END_FOR;
FOR HashIndex := UINT#0 TO UINT#7 DO
    DefinitionHash[HashIndex] := Producer.DefinitionHash[HashIndex];
    PrevDefinitionHash[HashIndex] := Producer.PrevDefinitionHash[HashIndex];
END_FOR;
HeadAbs := Producer.HeadAbs;
OldestAbs := Producer.OldestAbs;
LostCount := Producer.LostCount;
RunId := Producer.RunId;
EpochId := Producer.EpochId;
EpochFirstAbs := Producer.EpochFirstAbs;
Error := Error OR Producer.RingError;
END_FUNCTION_BLOCK
