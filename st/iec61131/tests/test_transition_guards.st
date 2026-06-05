FUNCTION_BLOCK OPENOT_TestTransitionGuards
VAR_OUTPUT
    Passed : BOOL;
    FinishBeforeDefinitionRejected : BOOL;
    DefinitionTwiceRejected : BOOL;
    BeginWhileOpenRejected : BOOL;
    FinishWithNoOpenRejected : BOOL;
    DefinitionWithNoOpenRejected : BOOL;
    CheckpointWhileOpenRejected : BOOL;
    HeldExecuteDidNotRepeat : BOOL;
END_VAR
VAR
    FinishProducer : OPENOT_Producer;
    TwiceProducer : OPENOT_Producer;
    OpenProducer : OPENOT_Producer;
    NoOpenProducer : OPENOT_Producer;
    CheckpointProducer : OPENOT_Producer;
    HeldProducer : OPENOT_Producer;
    DefHashOld : OPENOT_HASH8 := [
        BYTE#16#6F, BYTE#16#6C, BYTE#16#64, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#31
    ];
    DefHashNew : OPENOT_HASH8 := [
        BYTE#16#6E, BYTE#16#65, BYTE#16#77, BYTE#16#68,
        BYTE#16#61, BYTE#16#73, BYTE#16#68, BYTE#16#32
    ];
    Ran : BOOL;
END_VAR

IF NOT Ran THEN
    FinishProducer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#21);
    FinishProducer(Execute := FALSE, Op := UINT#0, SourceId := UDINT#21);
    FinishProducer(
        Execute := TRUE,
        Op := UINT#1,
        DefHashOld := DefHashOld,
        DefHashNew := DefHashNew,
        TransitionEpochId := ULINT#2);
    FinishProducer(Execute := FALSE, Op := UINT#1);
    FinishProducer(Execute := TRUE, Op := UINT#3);
    FinishBeforeDefinitionRejected := FinishProducer.Error AND (NOT FinishProducer.Done);

    TwiceProducer(
        Execute := TRUE,
        Op := UINT#1,
        DefHashOld := DefHashOld,
        DefHashNew := DefHashNew,
        TransitionEpochId := ULINT#2);
    TwiceProducer(Execute := FALSE, Op := UINT#1);
    TwiceProducer(Execute := TRUE, Op := UINT#2);
    TwiceProducer(Execute := FALSE, Op := UINT#2);
    TwiceProducer(Execute := TRUE, Op := UINT#2);
    DefinitionTwiceRejected := TwiceProducer.Error AND (NOT TwiceProducer.Done);

    OpenProducer(
        Execute := TRUE,
        Op := UINT#1,
        DefHashOld := DefHashOld,
        DefHashNew := DefHashNew,
        TransitionEpochId := ULINT#2);
    OpenProducer(Execute := FALSE, Op := UINT#1);
    OpenProducer(Execute := TRUE, Op := UINT#1);
    BeginWhileOpenRejected := OpenProducer.Error AND (NOT OpenProducer.Done);

    NoOpenProducer(Execute := TRUE, Op := UINT#3);
    FinishWithNoOpenRejected := NoOpenProducer.Error AND (NOT NoOpenProducer.Done);
    NoOpenProducer(Execute := FALSE, Op := UINT#3);
    NoOpenProducer(Execute := TRUE, Op := UINT#2);
    DefinitionWithNoOpenRejected := NoOpenProducer.Error AND (NOT NoOpenProducer.Done);

    CheckpointProducer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#21);
    CheckpointProducer(Execute := FALSE, Op := UINT#0, SourceId := UDINT#21);
    CheckpointProducer(
        Execute := TRUE,
        Op := UINT#1,
        DefHashOld := DefHashOld,
        DefHashNew := DefHashNew,
        TransitionEpochId := ULINT#2);
    CheckpointProducer(Execute := FALSE, Op := UINT#1);
    CheckpointProducer(Execute := TRUE, SourceId := UDINT#21, Checkpoint := TRUE);
    CheckpointWhileOpenRejected := CheckpointProducer.Error AND (NOT CheckpointProducer.Done);

    HeldProducer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#5);
    HeldProducer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#5);
    HeldExecuteDidNotRepeat :=
        (HeldProducer.EmissionCount = UINT#1) AND
        (HeldProducer.SourceNextSeqs[0] = ULINT#1);

    Ran := TRUE;
END_IF;

Passed :=
    FinishBeforeDefinitionRejected AND
    DefinitionTwiceRejected AND
    BeginWhileOpenRejected AND
    FinishWithNoOpenRejected AND
    DefinitionWithNoOpenRejected AND
    CheckpointWhileOpenRejected AND
    HeldExecuteDidNotRepeat;
END_FUNCTION_BLOCK
