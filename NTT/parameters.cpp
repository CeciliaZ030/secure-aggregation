// The main function receives two arguments argv[2]. 
// argv[1] is the vector size
// argv[2] is the number of the users
#include <NTL/BasicThreadPool.h>
#include <NTL/vector.h>
#include <NTL/ZZ.h>
#include <NTL/ZZ_pXFactoring.h>


using namespace std;
using namespace NTL;

using NTL::ZZ;

using NTL::Vec;

// Returns 1 if Number is a power of two greater than 1.
bool isPowerOfTwo(long Number){
    long n=2;
    while (1){
        if (n > Number)
            return 0;
        else if (n == Number)
            return 1;
        n *= 2;
    }
    return 0;
}

long whichRoot2(long Number){

    if (!isPowerOfTwo(Number)){
    cout<< Number << " is not a power of 2 or is 1"<<endl;
    return 0;
    }
    long n=2;
    long power=1;
    while (1){
        if (n == Number)
            return power;
        power += 1;    
        n *= 2;
    }
    return 0;
}

long whichRoot3(long Number){


    long n=3;
    long power=1;
    while (1){
        //TODO: check if equality should be considered
        if (n >= Number)
            return power;
        power += 1;    
        n *= 3;
    }
    return 0;
}

int main(int argc, char* argv[])
{
    if (argc < 3)
    {
        cout <<argc << "is too few for arguments"<<endl;
        cout <<"usage: " << "Vector_length Number_Users "<<endl;
        return 0;
    }
    
   ZZ theprime(3073700804129980417);       
   ZZ_p::init(theprime);
   ZZ_p rootTwo(1414118249734601779);
   Vec<ZZ_p> allRootTwo;
   allRootTwo.SetLength(21);
   allRootTwo[0] = 1;
   allRootTwo[20] = rootTwo;
   ZZ_p rootThree(308414859194273485);
   Vec<ZZ_p> allRootThree;
   allRootThree.SetLength(16);
   allRootThree[0] = 1;
   allRootThree[15] = rootThree;
   ZZ_p nth2(1048576);
   ZZ_p nth3(14348907);

   cout <<"The 62-bit prime is= " << theprime<< endl;
   cout <<" "<<nth2<<"th (2^20th) root of unity is=  " << rootTwo <<" (mod "<<theprime<<")" << endl;
   cout <<nth3<<"th (3^15th) root of unity is=   " << rootThree<<" (mod "<<theprime<<")" << endl;
  
   ZZ_p twos = rootTwo;
   long k=2;
   for (long i = 1 ; i < 20 ; i++ ){
       twos=twos*twos;
        allRootTwo[20-i] = twos;
       k=k*2;
   }
   ZZ_p threes = rootThree;
   k=3;
    for (long i = 1 ; i < 15 ; i++ ){
       threes=threes*threes*threes;
       allRootThree[15-i] = threes;
       k=k*3;
   }

   long vectorSize = atoi(argv[1]);
   long numUsers = atoi(argv[2]);
   long degree = numUsers/3;
   long numCrPrt = numUsers/6;
    cout << "vectorSize = "<< vectorSize << endl;
    cout << "numUsers = "<< numUsers << endl;
   if(!isPowerOfTwo(degree))
   {
       cout <<"degree before reduction= "<< degree <<endl;
    long degtmp= degree>>1;
    degree = 1 ;
    while(degtmp){
        degtmp>>=1;
        degree *= 2;
        
    }
   }

    cout << "degree = "<< degree <<"   (After reduction)"<< endl;
    cout << "numCrPrt = "<< numCrPrt << endl;
    cout << "block length = "<< degree - numCrPrt << endl;
    long whichr2 = whichRoot2(degree);
    long whichr3 = whichRoot3(degree);

    cout << allRootTwo[whichr2] << " is 2^"<<whichr2<<"th root of unity"<<endl;
    cout << allRootThree[whichr3] << " is 3^"<<whichr3<<"th root of unity"<<endl; 
 
	return 0;
	
}
