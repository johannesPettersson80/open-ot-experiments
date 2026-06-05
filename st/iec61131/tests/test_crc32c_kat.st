FUNCTION_BLOCK OPENOT_TestCrc32cKat
VAR_OUTPUT
    Passed : BOOL;
    MismatchIndex : UINT;
    ActualLength : UINT;
    ExpectedLength : UINT;
    ActualCrc : DWORD;
    ExpectedCrc : DWORD;
END_VAR
VAR
    Data : OPENOT_BYTE_BUFFER;
END_VAR

Data[0] := BYTE#16#31;
Data[1] := BYTE#16#32;
Data[2] := BYTE#16#33;
Data[3] := BYTE#16#34;
Data[4] := BYTE#16#35;
Data[5] := BYTE#16#36;
Data[6] := BYTE#16#37;
Data[7] := BYTE#16#38;
Data[8] := BYTE#16#39;

ActualLength := UINT#9;
ExpectedLength := UINT#9;
ActualCrc := OPENOT_Crc32c(Data := Data, Length := UINT#9);
ExpectedCrc := DWORD#16#E3069283;
Passed := ActualCrc = ExpectedCrc;

IF Passed THEN
    MismatchIndex := UINT#65535;
ELSE
    MismatchIndex := UINT#0;
END_IF;
END_FUNCTION_BLOCK
