FUNCTION_BLOCK OPENOT_TestTransitionWarm
VAR_OUTPUT
    Passed : BOOL;
    DataDuringTransitionRejected : BOOL;
    PostDataSeq : ULINT;
    SourceNextSeq : ULINT;
    EpochFirstAbsSet : BOOL;
    DefinitionHashOk : BOOL;
    PrevDefinitionHashOk : BOOL;
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
    LogOk : BOOL;
    SourceFound : BOOL;
    HashIndex : UINT;
    SourceIndex : UINT;
    EmissionCountBeforeError : UINT;
END_VAR

IF NOT Ran THEN
    OpsOk := TRUE;

    Producer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#21);
    OpsOk := OpsOk AND Producer.Done AND (NOT Producer.Error) AND (Producer.AssignedSeq = ULINT#0);
    Producer(Execute := FALSE, Op := UINT#0, SourceId := UDINT#21);

    Producer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#21);
    OpsOk := OpsOk AND Producer.Done AND (NOT Producer.Error) AND (Producer.AssignedSeq = ULINT#1);
    Producer(Execute := FALSE, Op := UINT#0, SourceId := UDINT#21);

    Producer(
        Execute := TRUE,
        Op := UINT#1,
        DefHashOld := DefHashOld,
        DefHashNew := DefHashNew,
        TransitionEpochId := ULINT#2);
    OpsOk := OpsOk AND Producer.Done AND (NOT Producer.Error) AND Producer.TransitionOpen;
    EmissionCountBeforeError := Producer.EmissionCount;
    Producer(Execute := FALSE, Op := UINT#1);

    Producer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#21);
    DataDuringTransitionRejected :=
        Producer.Error AND
        (NOT Producer.Done) AND
        (Producer.EmissionCount = EmissionCountBeforeError);
    Producer(Execute := FALSE, Op := UINT#0, SourceId := UDINT#21);

    Producer(Execute := TRUE, Op := UINT#2);
    OpsOk := OpsOk AND Producer.Done AND (NOT Producer.Error) AND Producer.DefinitionChangedEmitted;
    Producer(Execute := FALSE, Op := UINT#2);

    Producer(Execute := TRUE, Op := UINT#3);
    OpsOk :=
        OpsOk AND
        Producer.Done AND
        (NOT Producer.Error) AND
        (NOT Producer.TransitionOpen) AND
        (Producer.RunId = ULINT#1) AND
        (Producer.EpochId = ULINT#2);
    Producer(Execute := FALSE, Op := UINT#3);

    Producer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#21);
    PostDataSeq := Producer.AssignedSeq;
    OpsOk := OpsOk AND Producer.Done AND (NOT Producer.Error) AND (PostDataSeq = ULINT#3);

    Ran := TRUE;
END_IF;

SourceFound := FALSE;
SourceNextSeq := ULINT#0;
FOR SourceIndex := UINT#0 TO UINT#15 DO
    IF Producer.SourceUsed[SourceIndex] AND (Producer.SourceIds[SourceIndex] = UDINT#21) THEN
        SourceFound := TRUE;
        SourceNextSeq := Producer.SourceNextSeqs[SourceIndex];
    END_IF;
END_FOR;

DefinitionHashOk := TRUE;
PrevDefinitionHashOk := TRUE;
FOR HashIndex := UINT#0 TO UINT#7 DO
    DefinitionHashOk := DefinitionHashOk AND (Producer.DefinitionHash[HashIndex] = DefHashNew[HashIndex]);
    PrevDefinitionHashOk := PrevDefinitionHashOk AND (Producer.PrevDefinitionHash[HashIndex] = DefHashOld[HashIndex]);
END_FOR;

EpochFirstAbsSet := Producer.EpochFirstAbs > ULINT#0;

LogOk :=
    (Producer.EmissionCount = UINT#7) AND
    (Producer.Emissions[0].EventTypeId = UDINT#16#0003) AND
    (Producer.Emissions[0].RunId = ULINT#1) AND
    (Producer.Emissions[0].SourceId = UDINT#21) AND
    (Producer.Emissions[0].Seq = ULINT#0) AND
    (Producer.Emissions[1].EventTypeId = UDINT#16#0003) AND
    (Producer.Emissions[1].RunId = ULINT#1) AND
    (Producer.Emissions[1].SourceId = UDINT#21) AND
    (Producer.Emissions[1].Seq = ULINT#1) AND
    (Producer.Emissions[2].EventTypeId = UDINT#16#0102) AND
    (Producer.Emissions[2].RunId = ULINT#1) AND
    (Producer.Emissions[2].SourceId = UDINT#0) AND
    (Producer.Emissions[2].Seq = ULINT#0) AND
    (Producer.Emissions[3].EventTypeId = UDINT#16#80000108) AND
    (Producer.Emissions[3].RunId = ULINT#1) AND
    (Producer.Emissions[3].SourceId = UDINT#21) AND
    (Producer.Emissions[3].Seq = ULINT#2) AND
    (Producer.Emissions[4].EventTypeId = UDINT#16#0106) AND
    (Producer.Emissions[4].RunId = ULINT#1) AND
    (Producer.Emissions[4].SourceId = UDINT#0) AND
    (Producer.Emissions[4].Seq = ULINT#1) AND
    (Producer.Emissions[5].EventTypeId = UDINT#16#0101) AND
    (Producer.Emissions[5].RunId = ULINT#1) AND
    (Producer.Emissions[5].SourceId = UDINT#0) AND
    (Producer.Emissions[5].Seq = ULINT#2) AND
    (Producer.Emissions[6].EventTypeId = UDINT#16#0003) AND
    (Producer.Emissions[6].RunId = ULINT#1) AND
    (Producer.Emissions[6].SourceId = UDINT#21) AND
    (Producer.Emissions[6].Seq = ULINT#3);

Passed :=
    OpsOk AND
    DataDuringTransitionRejected AND
    LogOk AND
    SourceFound AND
    (Producer.PublishedRecordCount = ULINT#7) AND
    (SourceNextSeq = ULINT#4) AND
    (Producer.SourceTime = ULINT#1780000000000000007) AND
    EpochFirstAbsSet AND
    DefinitionHashOk AND
    PrevDefinitionHashOk;
END_FUNCTION_BLOCK
