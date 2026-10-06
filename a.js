var isHappy = function (n) {
  let inn = n;
  let input = inn.toString();
  let sp = [];
  let resultNum = 0;

  sp = input.split("");
  console.log(sp, input, resultNum);

  while (sp.length >= 2) {
    let addSumNum = 0;
    let currentUpdatedVal = [];
    for (let i = 0; i < sp.length; i++) {
      let a = parseInt(sp[i]);
      addSumNum = a * a + resultNum;
      inn = addSumNum;
      currentUpdatedVal = resultNum.toString().split("");
      console.log(sp, input, resultNum, "<><><><><><><><");
    }

    resultNum = addSumNum;
    sp = currentUpdatedVal;

    console.log(sp, input, resultNum, "<<<<<<<<<<<<<<<");
  }

  if (sp[0] === 1) return true;
  return false;
};

const n = 19;
console.log(isHappy(n));
