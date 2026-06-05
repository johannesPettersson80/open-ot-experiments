FUNCTION_BLOCK OPENOT_TestProducerSeq
VAR_OUTPUT
    Passed : BOOL;
    ASeq0 : ULINT;
    ASeq1 : ULINT;
    ASeq2 : ULINT;
    BSeq0 : ULINT;
    BSeq1 : ULINT;
    ANextSeq : ULINT;
    BNextSeq : ULINT;
    ReservedSourceRejected : BOOL;
    SourceTableFullRejected : BOOL;
    SourceCountAfterFill : UINT;
END_VAR
VAR
    Producer : OPENOT_Producer;
    ReservedProducer : OPENOT_Producer;
    FullProducer : OPENOT_Producer;
    Ran : BOOL;
    WritesOk : BOOL;
    FillOk : BOOL;
    AFound : BOOL;
    BFound : BOOL;
    Index : UINT;
    FillSource : UDINT;
END_VAR

IF NOT Ran THEN
    WritesOk := TRUE;
    FillOk := TRUE;

    Producer(Execute := TRUE, SourceId := UDINT#10);
    ASeq0 := Producer.AssignedSeq;
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error);

    Producer(Execute := TRUE, SourceId := UDINT#10);
    ASeq1 := Producer.AssignedSeq;
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error);

    Producer(Execute := TRUE, SourceId := UDINT#10);
    ASeq2 := Producer.AssignedSeq;
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error);

    Producer(Execute := TRUE, SourceId := UDINT#20);
    BSeq0 := Producer.AssignedSeq;
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error);

    Producer(Execute := TRUE, SourceId := UDINT#20);
    BSeq1 := Producer.AssignedSeq;
    WritesOk := WritesOk AND Producer.Done AND (NOT Producer.Error);

    ReservedProducer(Execute := TRUE, SourceId := UDINT#0);
    ReservedSourceRejected := ReservedProducer.Error AND (NOT ReservedProducer.Done);

    FOR Index := UINT#0 TO UINT#15 DO
        FillSource := UINT_TO_UDINT(Index) + UDINT#1;
        FullProducer(Execute := TRUE, SourceId := FillSource);
        FillOk :=
            FillOk AND
            FullProducer.Done AND
            (NOT FullProducer.Error) AND
            (NOT FullProducer.SourceTableFull);
    END_FOR;

    SourceCountAfterFill := FullProducer.SourceCount;
    FullProducer(Execute := TRUE, SourceId := UDINT#17);
    SourceTableFullRejected :=
        FullProducer.Error AND
        FullProducer.SourceTableFull AND
        (NOT FullProducer.Done);

    Ran := TRUE;
END_IF;

AFound := FALSE;
BFound := FALSE;
ANextSeq := ULINT#0;
BNextSeq := ULINT#0;
FOR Index := UINT#0 TO UINT#15 DO
    IF Producer.SourceUsed[Index] AND (Producer.SourceIds[Index] = UDINT#10) THEN
        ANextSeq := Producer.SourceNextSeqs[Index];
        AFound := TRUE;
    END_IF;
    IF Producer.SourceUsed[Index] AND (Producer.SourceIds[Index] = UDINT#20) THEN
        BNextSeq := Producer.SourceNextSeqs[Index];
        BFound := TRUE;
    END_IF;
END_FOR;

Passed :=
    WritesOk AND
    (ASeq0 = ULINT#0) AND
    (ASeq1 = ULINT#1) AND
    (ASeq2 = ULINT#2) AND
    (BSeq0 = ULINT#0) AND
    (BSeq1 = ULINT#1) AND
    AFound AND
    BFound AND
    (ANextSeq = ULINT#3) AND
    (BNextSeq = ULINT#2) AND
    (Producer.SourceCount = UINT#2) AND
    ReservedSourceRejected AND
    FillOk AND
    (SourceCountAfterFill = UINT#16) AND
    SourceTableFullRejected;
END_FUNCTION_BLOCK
