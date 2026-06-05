FUNCTION_BLOCK OPENOT_TestPublishedRecordCountDurable
VAR_OUTPUT
    Passed : BOOL;
    PublishedRecordCount : ULINT;
    EmissionCount : UINT;
    FinalAssignedSeq : ULINT;
END_VAR
VAR
    Producer : OPENOT_Producer;
    Ran : BOOL;
    WritesOk : BOOL;
    Index : UINT;
    ExpectedSeq : ULINT;
END_VAR

IF NOT Ran THEN
    WritesOk := TRUE;
    ExpectedSeq := ULINT#0;

    FOR Index := UINT#0 TO UINT#39 DO
        Producer(Execute := TRUE, Op := UINT#0, SourceId := UDINT#77);
        FinalAssignedSeq := Producer.AssignedSeq;
        WritesOk :=
            WritesOk AND
            Producer.Done AND
            (NOT Producer.Error) AND
            (Producer.AssignedSeq = ExpectedSeq);
        Producer(Execute := FALSE, Op := UINT#0, SourceId := UDINT#77);
        ExpectedSeq := ExpectedSeq + ULINT#1;
    END_FOR;

    PublishedRecordCount := Producer.PublishedRecordCount;
    EmissionCount := Producer.EmissionCount;
    Ran := TRUE;
END_IF;

Passed :=
    WritesOk AND
    (PublishedRecordCount = ULINT#40) AND
    (EmissionCount = UINT#32) AND
    (FinalAssignedSeq = ULINT#39) AND
    (Producer.PublishedRecordCount = ULINT#40) AND
    (Producer.EmissionCount = UINT#32) AND
    (Producer.SourceCount = UINT#1);
END_FUNCTION_BLOCK
