FUNCTION_BLOCK OPENOT_TestProducerCheckpoint
VAR_OUTPUT
    Passed : BOOL;
    ProducedCount : ULINT;
    NextSeqAfterCheckpoint : ULINT;
    EnvelopeSeqIsFive : BOOL;
    PayloadProducedCountIsFive : BOOL;
    CheckpointLength : UINT;
    CheckpointStartAbs : ULINT;
END_VAR
VAR
    Producer : OPENOT_Producer;
    Ran : BOOL;
    WritesOk : BOOL;
    SourceFound : BOOL;
    Index : UINT;
END_VAR

IF NOT Ran THEN
    WritesOk := TRUE;

    Producer(Execute := TRUE, SourceId := UDINT#88, Checkpoint := FALSE);
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error) AND (Producer.AssignedSeq = ULINT#0);
    Producer(Execute := FALSE, SourceId := UDINT#88, Checkpoint := FALSE);
    Producer(Execute := TRUE, SourceId := UDINT#88, Checkpoint := FALSE);
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error) AND (Producer.AssignedSeq = ULINT#1);
    Producer(Execute := FALSE, SourceId := UDINT#88, Checkpoint := FALSE);
    Producer(Execute := TRUE, SourceId := UDINT#88, Checkpoint := FALSE);
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error) AND (Producer.AssignedSeq = ULINT#2);
    Producer(Execute := FALSE, SourceId := UDINT#88, Checkpoint := FALSE);
    Producer(Execute := TRUE, SourceId := UDINT#88, Checkpoint := FALSE);
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error) AND (Producer.AssignedSeq = ULINT#3);
    Producer(Execute := FALSE, SourceId := UDINT#88, Checkpoint := FALSE);
    Producer(Execute := TRUE, SourceId := UDINT#88, Checkpoint := FALSE);
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error) AND (Producer.AssignedSeq = ULINT#4);
    Producer(Execute := FALSE, SourceId := UDINT#88, Checkpoint := FALSE);

    Producer(Execute := TRUE, SourceId := UDINT#88, Checkpoint := TRUE);
    ProducedCount := Producer.ProducedCount;
    CheckpointLength := UINT#56;
    CheckpointStartAbs := Producer.LastStartAbs;

    EnvelopeSeqIsFive :=
        (Producer.Ring[24] = BYTE#16#05) AND
        (Producer.Ring[25] = BYTE#16#00) AND
        (Producer.Ring[26] = BYTE#16#00) AND
        (Producer.Ring[27] = BYTE#16#00) AND
        (Producer.Ring[28] = BYTE#16#00) AND
        (Producer.Ring[29] = BYTE#16#00) AND
        (Producer.Ring[30] = BYTE#16#00) AND
        (Producer.Ring[31] = BYTE#16#00);

    PayloadProducedCountIsFive :=
        (Producer.Ring[44] = BYTE#16#05) AND
        (Producer.Ring[45] = BYTE#16#00) AND
        (Producer.Ring[46] = BYTE#16#00) AND
        (Producer.Ring[47] = BYTE#16#00) AND
        (Producer.Ring[48] = BYTE#16#00) AND
        (Producer.Ring[49] = BYTE#16#00) AND
        (Producer.Ring[50] = BYTE#16#00) AND
        (Producer.Ring[51] = BYTE#16#00);

    Ran := TRUE;
END_IF;

SourceFound := FALSE;
NextSeqAfterCheckpoint := ULINT#0;
FOR Index := UINT#0 TO UINT#15 DO
    IF Producer.SourceUsed[Index] AND (Producer.SourceIds[Index] = UDINT#88) THEN
        SourceFound := TRUE;
        NextSeqAfterCheckpoint := Producer.SourceNextSeqs[Index];
    END_IF;
END_FOR;

Passed :=
    WritesOk AND
    Producer.Done AND
    Producer.CheckpointDone AND
    (NOT Producer.Error) AND
    (ProducedCount = ULINT#5) AND
    (CheckpointStartAbs = ULINT#256) AND
    (Producer.LastEndAbs = ULINT#312) AND
    (CheckpointLength = UINT#56) AND
    EnvelopeSeqIsFive AND
    PayloadProducedCountIsFive AND
    SourceFound AND
    (Producer.PublishedRecordCount = ULINT#6) AND
    (NextSeqAfterCheckpoint = ULINT#6);
END_FUNCTION_BLOCK
