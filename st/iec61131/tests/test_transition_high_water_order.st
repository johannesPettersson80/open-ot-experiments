FUNCTION_BLOCK OPENOT_TestTransitionHighWaterOrder
VAR_OUTPUT
    Passed : BOOL;
    FirstHighWaterSource : UDINT;
    SecondHighWaterSource : UDINT;
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
    OpsOk : BOOL;
END_VAR

IF NOT Ran THEN
    OpsOk := TRUE;

    Producer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#30);
    OpsOk := OpsOk AND Producer.Done AND (NOT Producer.Error);
    Producer(Execute := FALSE, Op := UINT#0, SourceId := UDINT#30);

    Producer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#10);
    OpsOk := OpsOk AND Producer.Done AND (NOT Producer.Error);
    Producer(Execute := FALSE, Op := UINT#0, SourceId := UDINT#10);

    Producer(
        Execute := TRUE,
        Op := UINT#1,
        DefHashOld := DefHashOld,
        DefHashNew := DefHashNew,
        TransitionEpochId := ULINT#2);
    OpsOk := OpsOk AND Producer.Done AND (NOT Producer.Error);

    FirstHighWaterSource := Producer.Emissions[3].SourceId;
    SecondHighWaterSource := Producer.Emissions[4].SourceId;
    Ran := TRUE;
END_IF;

Passed :=
    OpsOk AND
    (Producer.EmissionCount = UINT#5) AND
    (Producer.PublishedRecordCount = ULINT#5) AND
    (Producer.Emissions[2].EventTypeId = UDINT#16#0102) AND
    (Producer.Emissions[3].EventTypeId = UDINT#16#80000108) AND
    (Producer.Emissions[3].SourceId = UDINT#10) AND
    (Producer.Emissions[3].Seq = ULINT#1) AND
    (Producer.Emissions[4].EventTypeId = UDINT#16#80000108) AND
    (Producer.Emissions[4].SourceId = UDINT#30) AND
    (Producer.Emissions[4].Seq = ULINT#1);
END_FUNCTION_BLOCK
